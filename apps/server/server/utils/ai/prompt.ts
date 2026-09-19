// '/types' subpath, NOT the package root - the root barrel also exports every Control*.vue
// component, and unlike a type-only import (erased at build time), this is a real value import:
// Nitro's server bundler would have to actually resolve and parse those .vue SFCs outside the
// Vue/Vite pipeline, which crashes its build (confirmed live - "Expression expected" from Rollup
// trying to parse ControlBooleanBase.vue as plain JS). '/types' is exactly the DOM/component-free
// subpath the codebase already established for this reason (see apps/server/shared/types/protocol.ts).
import { CONTROL_ICONS } from '@vrc-osc-toolkit/shared-ui/types'

import type { PromptProfile } from './profiles'
import type { AiParameterInput } from './types'

// The exact same icon set the client's own picker offers (see packages/shared-ui's
// constants/controlIcons.ts) - not a separate AI-only list, so a control this feature creates
// always uses an icon the user could equally have picked by hand in ControlModal.vue, and there's
// only one list to keep accurate/extended over time. A closed vocabulary, not "any Iconify name
// the model thinks of": we have no icon registry to validate an arbitrary name against server-side,
// so an invented one would just silently render as a blank/broken icon client-side. Keeping it as
// a schema `enum` (see below) means constrained decoding guarantees a valid pick, not a regex
// allow-list check after the fact.
export const ICON_CHOICES = CONTROL_ICONS.map((i) => i.icon)

// "label (icon-value)" per entry, grouped by category - handed to the model in the prompt so it
// picks by matching a control's real-world meaning to a label, not by guessing what an opaque
// Iconify string like "game-icons:fox-tail" probably looks like.
function iconChoicesForPrompt(): string {
  const byCategory = new Map<string, string[]>()

  for (const { category, label, icon } of CONTROL_ICONS) {
    const entries = byCategory.get(category) ?? []
    entries.push(`${label} (${icon})`)
    byCategory.set(category, entries)
  }

  return [...byCategory.entries()].map(([category, entries]) => `${category}: ${entries.join(', ')}`).join('\n')
}

// Shared between both providers - Groq (strict json_schema mode) additionally requires every
// property to be listed under `required` and `additionalProperties: false` at every level, which
// Gemini's `responseSchema` neither needs nor accepts, so each provider adapts this base shape
// rather than sending it as-is (see providers/groq.ts, providers/gemini.ts). `type`'s enum is
// restricted to the calling profile's `allowedTypes` - a hard guarantee (not just prompt wording)
// that e.g. a 'light' request can't come back with a 'boolean-enum' even if the model wanted to.
export function buildControlSuggestionSchema(profile: PromptProfile) {
  return {
    type: 'object',
    properties: {
      groups: {
        type: 'array',
        items: {
          type: 'object',
          properties: {
            name: { type: 'string' },
            controls: {
              type: 'array',
              items: {
                type: 'object',
                properties: {
                  name: { type: 'string' },
                  type: { type: 'string', enum: [...profile.allowedTypes] },
                  address: { type: 'string' },
                  addresses: { type: 'array', items: { type: 'string' } },
                  icon: { type: 'string', enum: [...ICON_CHOICES] },
                  options: {
                    type: 'array',
                    items: {
                      type: 'object',
                      properties: {
                        name: { type: 'string' },
                        icon: { type: 'string', enum: [...ICON_CHOICES] },
                        value: { type: 'number' },
                        trueAddresses: { type: 'array', items: { type: 'string' } },
                        falseAddresses: { type: 'array', items: { type: 'string' } }
                      },
                      required: ['name']
                    }
                  }
                },
                required: ['name', 'type']
              }
            }
          },
          required: ['name', 'controls']
        }
      }
    },
    required: ['groups']
  } as const
}

const TYPE_GUIDANCE: Record<PromptProfile['allowedTypes'][number], string> = {
  boolean: `- 'Bool' kind, standalone -> 'boolean'. Set "address" to that parameter's address.`,
  slider: `- 'Float' kind -> always 'slider'. Set "address".`,
  'step-enum': `- 'Int' kind -> 'slider' by default, OR 'step-enum' with an "options" array (each option needs "name" and "value") ONLY if you're genuinely confident you can guess a short, meaningful, small (2-8) set of named values from the parameter's name/context - prefer 'slider' when unsure. Set "address".`,
  'boolean-group': `- 'boolean-group': several separate Bool parameters that clearly always get toggled on together as ONE conceptual thing (e.g. a multi-piece outfit that's really one look, each piece its own address, but there's no reason you'd ever want only some of them on). Set "addresses" to all of them - every one is set to the same value in a single activation. Use sparingly, only when the names make the "one unit" relationship obvious.`,
  'boolean-enum': `- 'boolean-enum': a MUTUALLY EXCLUSIVE set of Bool parameters that act like a radio-button choice between alternatives in the same "slot" (e.g. three different top styles where the avatar creator clearly only intends one at a time). Set "options" to one entry per choice, each with "name" and "trueAddresses" (that choice's own address, usually just one) and "falseAddresses" (every OTHER address in this same mutually-exclusive set, so picking one turns the rest off). Only use this when exclusivity is genuinely obvious from the names - you cannot see the avatar's actual animator logic, so when in doubt use plain separate 'boolean' controls instead. Getting this wrong (forcing exclusivity that isn't real) is worse than leaving parameters as independent toggles.`
}

export function buildSystemPrompt(profile: PromptProfile, clusterHints: string[]): string {
  const typeGuidance = profile.allowedTypes.map((type) => TYPE_GUIDANCE[type]).join('\n')

  const hintsSection = clusterHints.length
    ? `\n\nAutomated pre-analysis found these possible relationships between parameters - each is only a candidate, verify it makes sense before using it, and ignore any that don't:\n${clusterHints.map((h) => `- ${h}`).join('\n')}`
    : ''

  return `You are helping organize a VRChat avatar's OSC parameters into a quick-menu-style control panel for a companion desktop app.

You will be given a JSON array of parameters already pre-filtered by the app (known-noisy PhysBone/VRCFury/tracking namespaces are already removed). For each one you get: name (raw OSC parameter name), address (OSC address - copy it back EXACTLY, never invent or modify one), kind ('Bool' | 'Float' | 'Int').

Your job:
1. Decide which parameters are worth exposing as a manual control. Omit any that still look like automatic/internal/status parameters rather than something a person would intentionally toggle or adjust (tracking state, gesture/viseme mirrors, velocity/physics, sync/internal plumbing, anything clearly read-only).
2. Group the rest into logical, human-friendly groups by theme (e.g. "Clothing", "Hair", "Lighting"). Prefer several focused groups over one giant one; a group with a single obviously-standalone control is fine.
3. Give each control a short, readable display name (clean up underscores/casing; you don't need to keep the raw parameter name verbatim if a cleaner one is obvious).
4. Pick a control "type" per parameter, or per small cluster of related parameters:
${typeGuidance}
5. Give every control (and every 'boolean-enum' option) an "icon" - pick the single closest match for what it actually does, using the value in parentheses (copied verbatim, never invent your own), from this list:
${iconChoicesForPrompt()}
6. Every address/addresses/trueAddresses/falseAddresses value must be copied verbatim from the input - never fabricate one, never modify one.${hintsSection}

Respond with ONLY the JSON object matching the given schema. No prose, no markdown fences.`
}

export function buildUserPrompt(avatarName: string, parameters: AiParameterInput[]): string {
  return `Avatar: ${avatarName}\n\nParameters (${parameters.length}):\n${JSON.stringify(parameters)}`
}
