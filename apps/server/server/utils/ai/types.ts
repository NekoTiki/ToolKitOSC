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
// model only ever gets to choose between the three types that a plain OSC parameter can honestly
// be turned into (boolean/slider/step-enum); OpenShock/Intiface/preset controls aren't something
// it has any basis to invent.
export interface AiControlSuggestion {
  name: string
  address: string
  type: 'boolean' | 'slider' | 'step-enum'
  options?: { name: string; value: number }[]
}

export interface AiGroupSuggestion {
  name: string
  controls: AiControlSuggestion[]
}

export interface AiSuggestionResult {
  groups: AiGroupSuggestion[]
}

export interface AiProvider {
  id: 'groq' | 'gemini'
  label: string
  isConfigured: () => boolean
  suggestControlGroups: (avatarName: string, parameters: AiParameterInput[]) => Promise<AiSuggestionResult>
}
