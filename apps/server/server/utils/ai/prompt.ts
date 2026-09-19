import type { AiParameterInput } from './types'

// Shared between both providers - Groq (strict json_schema mode) additionally requires every
// property to be listed under `required` and `additionalProperties: false` at every level, which
// Gemini's `responseSchema` neither needs nor accepts, so each provider adapts this base shape
// rather than sending it as-is (see providers/groq.ts, providers/gemini.ts).
export const CONTROL_SUGGESTION_SCHEMA = {
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
                address: { type: 'string' },
                type: { type: 'string', enum: ['boolean', 'slider', 'step-enum'] },
                options: {
                  type: 'array',
                  items: {
                    type: 'object',
                    properties: {
                      name: { type: 'string' },
                      value: { type: 'number' }
                    },
                    required: ['name', 'value']
                  }
                }
              },
              required: ['name', 'address', 'type']
            }
          }
        },
        required: ['name', 'controls']
      }
    }
  },
  required: ['groups']
} as const

export const SYSTEM_PROMPT = `You are helping organize a VRChat avatar's OSC parameters into a quick-menu-style control panel for a companion desktop app.

You will be given a JSON array of parameters already pre-filtered by the app (known-noisy PhysBone/VRCFury/tracking namespaces are already removed). For each one you get: name (raw OSC parameter name), address (OSC address - copy it back EXACTLY, never invent or modify one), kind ('Bool' | 'Float' | 'Int').

Your job:
1. Decide which parameters are worth exposing as a manual control. Omit any that still look like automatic/internal/status parameters rather than something a person would intentionally toggle or adjust (tracking state, gesture/viseme mirrors, velocity/physics, sync/internal plumbing, anything clearly read-only).
2. Group the rest into logical, human-friendly groups by theme (e.g. "Clothing", "Hair", "Lighting"). Prefer several focused groups over one giant one; a group with a single obviously-standalone control is fine.
3. Give each control a short, readable display name (clean up underscores/casing; you don't need to keep the raw parameter name verbatim if a cleaner one is obvious).
4. Pick the control type based on kind: 'Bool' -> always 'boolean'. 'Float' -> always 'slider'. 'Int' -> 'slider' by default, OR 'step-enum' with an "options" array ONLY if you're genuinely confident you can guess a short, meaningful, small (2-8) set of named values from the parameter's name/context - never invent an options list you're not confident about, prefer 'slider' when unsure.
5. address must be copied verbatim from the input - never fabricate one.

Respond with ONLY the JSON object matching the given schema. No prose, no markdown fences.`

export function buildUserPrompt(avatarName: string, parameters: AiParameterInput[]): string {
  return `Avatar: ${avatarName}\n\nParameters (${parameters.length}):\n${JSON.stringify(parameters)}`
}
