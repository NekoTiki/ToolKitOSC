// '/types' subpath, NOT the package root - the root barrel also exports every Control*.vue
// component, and unlike a type-only import (erased at build time), this is a real value import:
// Nitro's server bundler would have to actually resolve and parse those .vue SFCs outside the
// Vue/Vite pipeline, which crashes its build (confirmed live - "Expression expected" from Rollup
// trying to parse ControlBooleanBase.vue as plain JS). '/types' is exactly the DOM/component-free
// subpath the codebase already established for this reason (see apps/server/shared/types/protocol.ts).
import { CONTROL_ICONS } from '@toolkitosc/shared-ui/types'

import type { PromptProfile, PromptProfileId } from './profiles'
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

// Guidance per control type varies by profile, not just which types are allowed - Light/Balanced/
// Heavy used to share the exact same wording for boolean-group/boolean-enum, with nothing telling
// Heavy to actually be more assertive about finding them than Balanced (its only real differences
// were the extra cluster-hints pass and a bigger token budget). Balanced stays reasonably
// cautious; Heavy is explicitly told this kind of structure-finding is its main job.
function typeGuidance(type: PromptProfile['allowedTypes'][number], profileId: PromptProfileId): string {
  switch (type) {
    case 'boolean':
      return `- 'Bool' kind, standalone -> 'boolean'. Set "address" to that parameter's address.`

    case 'slider':
      return `- 'Float' kind -> always 'slider'. Set "address".`

    case 'step-enum':
      return profileId === 'heavy'
        ? `- 'Int' kind -> prefer 'step-enum' with a named "options" array (each needs "name" and "value") whenever you can infer a plausible small (2-8) set of values from the parameter's name/context - a plain 'slider' is a fallback for when there's genuinely nothing to name the steps after, not the default. Set "address".`
        : `- 'Int' kind -> 'slider' by default, OR 'step-enum' with an "options" array (each option needs "name" and "value") when you can confidently name a short, meaningful, small (2-8) set of values from the parameter's name/context - prefer 'slider' only when the values genuinely aren't guessable. Set "address".`

    case 'boolean-group':
      return profileId === 'heavy'
        ? `- 'boolean-group': several separate Bool parameters that always get toggled on together as ONE conceptual thing (e.g. a multi-piece outfit that's really one look, each piece its own address, but there's no reason you'd ever want only some of them on). Set "addresses" to all of them - every one is set to the same value in a single activation. Actively look for this pattern across the WHOLE parameter list, not just within one obvious group - prefer finding a real "one unit" relationship over leaving related parameters as separate, disconnected toggles.`
        : `- 'boolean-group': several separate Bool parameters that clearly always get toggled on together as ONE conceptual thing (e.g. a multi-piece outfit that's really one look, each piece its own address, but there's no reason you'd ever want only some of them on). Set "addresses" to all of them - every one is set to the same value in a single activation. Use it when the "one unit" relationship is reasonably clear from the names.`

    case 'boolean-enum':
      return profileId === 'heavy'
        ? `- 'boolean-enum': a MUTUALLY EXCLUSIVE set of Bool parameters that act like a radio-button choice between alternatives in the same "slot" (e.g. three different top styles, or several expression variants, where only one is ever meant to be on at once). Set "options" to one entry per choice, each with "name" and "trueAddresses" (that choice's own address, usually just one) and "falseAddresses" (every OTHER address in this same mutually-exclusive set, so picking one turns the rest off). This is Heavy mode's main job: actively scan the ENTIRE parameter list for numbered/suffixed siblings (_1/_2/_3, Left/Right/Center, named variants of the same base concept) and treat them as exclusive-choice candidates by default whenever the names plausibly describe alternatives in the same slot - don't settle for a flat list of separate booleans when a real choice-set is sitting right there in the names. You still cannot see the actual animator logic, so don't invent a relationship the names don't support at all - but between "found it" and "played it safe," lean toward finding it.`
        : `- 'boolean-enum': a MUTUALLY EXCLUSIVE set of Bool parameters that act like a radio-button choice between alternatives in the same "slot" (e.g. three different top styles where the avatar creator clearly only intends one at a time). Set "options" to one entry per choice, each with "name" and "trueAddresses" (that choice's own address, usually just one) and "falseAddresses" (every OTHER address in this same mutually-exclusive set, so picking one turns the rest off). Use it when exclusivity is reasonably clear from the names - you cannot see the avatar's actual animator logic, so when genuinely unsure use plain separate 'boolean' controls instead.`
  }
}

// Only Heavy gets this - its explicit charter beyond Balanced. Placed as its own emphasized block
// rather than folded into the numbered steps, so it reads as a standing instruction to check
// against at the end, not just one more bullet to skim past.
const HEAVY_DIRECTIVE = `

HEAVY MODE: your job beyond a normal pass is finding real toggle-group and exclusive-choice structure instead of leaving everything as a flat list of separate booleans. Before finishing, go back over every cluster of 2+ Bool parameters that share a naming pattern (a common prefix/stem, a numbered or Left/Right/named suffix) and explicitly decide whether it's a boolean-group, a boolean-enum, or genuinely independent toggles - don't skip that check just because it takes more thought. An occasional wrong grouping call is an acceptable cost for this profile; leaving an obvious structure ungrouped is not.`

function hintsSection(profileId: PromptProfileId, clusterHints: string[]): string {
  if (!clusterHints.length) return ''

  const intro =
    profileId === 'heavy'
      ? 'Automated pre-analysis found these likely relationships between parameters - treat them as strong candidates and apply them by default unless the names actively contradict it, not as suggestions to second-guess away'
      : "Automated pre-analysis found these possible relationships between parameters - each is only a candidate, verify it makes sense before using it, and ignore any that don't"

  return `\n\n${intro}:\n${clusterHints.map((h) => `- ${h}`).join('\n')}`
}

export function buildSystemPrompt(profile: PromptProfile, clusterHints: string[]): string {
  const typeGuidanceText = profile.allowedTypes.map((type) => typeGuidance(type, profile.id)).join('\n')
  const heavyDirective = profile.id === 'heavy' ? HEAVY_DIRECTIVE : ''

  return `You are helping organize a VRChat avatar's OSC parameters into a quick-menu-style control panel for a companion desktop app.

You will be given a JSON array of parameters already pre-filtered by the app (known-noisy PhysBone/VRCFury/tracking namespaces are already removed). For each one you get: name (raw OSC parameter name), address (OSC address - copy it back EXACTLY, never invent or modify one), kind ('Bool' | 'Float' | 'Int').

Your job:
1. Decide which parameters are worth exposing as a manual control. Include everything a person could plausibly want to adjust by hand - clothing/accessory toggles, color/material choices, size/physics sliders, expression or pose toggles, lighting/effect toggles, and so on. Only omit parameters that clearly look automatic/internal/status (tracking state, gesture/viseme mirrors, velocity/physics feedback, sync/internal plumbing, anything clearly read-only or driven by animation rather than user choice). When genuinely unsure whether something is worth exposing, lean toward including it - a control the user doesn't need is easy to ignore, one that's missing entirely isn't.
2. Group the rest into logical, human-friendly groups by theme (e.g. "Clothing", "Hair", "Lighting", "Body", "Accessories", "Effects"). Prefer several focused groups over one giant catch-all - a real avatar usually has more structure in it than one flat list suggests. A group with a single obviously-standalone control is fine.
3. Give each control a short, readable display name (clean up underscores/casing; you don't need to keep the raw parameter name verbatim if a cleaner one is obvious).
4. Pick a control "type" per parameter, or per small cluster of related parameters:
${typeGuidanceText}
5. Give every control (and every 'boolean-enum' option) an "icon" - pick the single closest match for what it actually does, using the value in parentheses (copied verbatim, never invent your own), from this list:
${iconChoicesForPrompt()}
6. Every address/addresses/trueAddresses/falseAddresses value must be copied verbatim from the input - never fabricate one, never modify one.${hintsSection(profile.id, clusterHints)}${heavyDirective}

Respond with ONLY the JSON object matching the given schema. No prose, no markdown fences. Write each object property exactly once - never repeat a property name within the same object, even if you want to revise an earlier value; get it right the first time instead.`
}

export function buildUserPrompt(avatarName: string, parameters: AiParameterInput[]): string {
  return `Avatar: ${avatarName}\n\nParameters (${parameters.length}):\n${JSON.stringify(parameters)}`
}
