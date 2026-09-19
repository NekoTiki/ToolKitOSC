// One entry per avatar parameter the client has already decided is worth showing the AI - the
// client filters with its own existing noisy-address denylist + the user's per-avatar excluded
// list (see apps/client's usePresets.ts `includedParameters`) before this ever reaches the server,
// so nothing here re-implements that filtering.
export interface AiParameterInput {
  name: string
  address: string
  kind: 'Bool' | 'Float' | 'Int'
}

// What every provider must produce, before server-side validation turns it into real
// ControlGroup[]/ControlType[] (see normalize.ts). Deliberately narrower than ControlType - the
// model only ever gets to choose between the five types that a plain OSC parameter (or a small
// cluster of them) can honestly be turned into; OpenShock/Intiface/preset controls aren't
// something it has any basis to invent.
//
// Every field a type doesn't use is just left undefined by the model - flattened rather than a
// true discriminated union because neither Groq's strict json_schema mode nor Gemini's
// responseSchema reliably supports a schema shape that actually varies by a sibling field's value
// (see prompt.ts's CONTROL_SUGGESTION_SCHEMA), the same reason `options` was already optional
// before this. normalize.ts is what actually enforces which fields matter per `type`.
export interface AiControlSuggestion {
  name: string
  type: 'boolean' | 'slider' | 'step-enum' | 'boolean-group' | 'boolean-enum'
  // boolean | slider
  address?: string
  // step-enum: value per option. boolean-enum: which addresses that option drives true/false.
  options?: {
    name: string
    icon?: string
    value?: number
    trueAddresses?: string[]
    falseAddresses?: string[]
  }[]
  // boolean-group: every address here is set to the same value in one activation.
  addresses?: string[]
  // Iconify name, e.g. "i-lucide-shirt" - see prompt.ts's ICON_CHOICES for the allowed set.
  icon?: string
}

export interface AiGroupSuggestion {
  name: string
  controls: AiControlSuggestion[]
}

export interface AiSuggestionResult {
  groups: AiGroupSuggestion[]
}

export type AiProviderId = 'groq' | 'gemini' | 'openrouter' | 'cerebras' | 'cloudflare'

export interface AiProvider {
  id: AiProviderId
  label: string
  isConfigured: () => boolean
  // profile is the PromptProfile (see profiles.ts) driving allowed types/token budget/model choice
  // for this call - typed loosely here (not importing PromptProfile) to avoid a circular import
  // between profiles.ts (which needs AiProviderId from this file) and types.ts.
  suggestControlGroups: (
    avatarName: string,
    parameters: AiParameterInput[],
    profile: import('./profiles').PromptProfile
  ) => Promise<AiSuggestionResult>
}
