import type { ControlTypes } from '@vrc-osc-toolkit/shared-ui'

import type { AiProviderId } from './types'

export type PromptProfileId = 'light' | 'balanced' | 'heavy'

export interface PromptProfile {
  id: PromptProfileId
  label: string
  description: string
  // Restricts the JSON schema's `type` enum, not just the prompt wording - a hard guarantee the
  // model can't route around (matters most for Groq's strict mode), not a suggestion.
  allowedTypes: Extract<ControlTypes, 'boolean' | 'slider' | 'step-enum' | 'boolean-group' | 'boolean-enum'>[]
  maxCompletionTokens: number
  // Whether to run detectClusterHints() (see clustering.ts) and hand its output to the model as
  // candidate boolean-group/boolean-enum hints - only worth the extra pre-pass when the profile
  // actually allows those types and wants to be assertive about finding them.
  useClusterHints: boolean
  // What one generation at this profile spends from the shared per-(account, provider) daily
  // credit pool (see rateLimit.ts / ACCOUNT_DAILY_CREDITS) - a real cost model, not a per-profile
  // request cap: credits are fungible, so 20 Light generations, 10 Balanced ones, 5 Heavy ones, or
  // any mix that adds up to the same pool, all cost the same account the same budget.
  creditCost: number
}

export const PROMPT_PROFILES: Record<PromptProfileId, PromptProfile> = {
  light: {
    id: 'light',
    label: 'Light',
    description: 'Fast and safe - toggles and sliders only, no relationship guessing between parameters.',
    allowedTypes: ['boolean', 'slider', 'step-enum'],
    maxCompletionTokens: 8000,
    useClusterHints: false,
    creditCost: 1
  },
  balanced: {
    id: 'balanced',
    label: 'Balanced',
    description: 'Uses every control type where the relationship between parameters is obvious from their names.',
    allowedTypes: ['boolean', 'slider', 'step-enum', 'boolean-group', 'boolean-enum'],
    maxCompletionTokens: 12000,
    useClusterHints: false,
    creditCost: 2
  },
  heavy: {
    id: 'heavy',
    label: 'Heavy',
    description: 'Most thorough - pre-detects likely toggle-group/exclusive-choice candidates and pushes the model to use the full control surface. Slower, costs more, higher chance of a wrong grouping call.',
    allowedTypes: ['boolean', 'slider', 'step-enum', 'boolean-group', 'boolean-enum'],
    maxCompletionTokens: 16000,
    useClusterHints: true,
    creditCost: 4
  }
}

// The shared pool every profile spends from, per (discord account, provider) per day - matches
// the old per-profile limits' implied ratio (20/10/5, i.e. cost ratio 1:2:4) so nothing about the
// total daily budget actually changed, just how it's allocated (fungibly, across profiles,
// instead of siloed per profile).
export const ACCOUNT_DAILY_CREDITS = 20

export function getPromptProfile(id: unknown): PromptProfile | undefined {
  return typeof id === 'string' && id in PROMPT_PROFILES
    ? PROMPT_PROFILES[id as PromptProfileId]
    : undefined
}

// Used for accounts without model-select permission (see utils/ai/access.ts) - NUXT_AI_DEFAULT_PROFILE
// falls back to 'balanced' if unset or misconfigured, rather than failing a generation over it.
export function resolveDefaultProfile(): PromptProfile {
  return getPromptProfile(useRuntimeConfig().aiDefaultProfile) ?? PROMPT_PROFILES.balanced
}

// The model choice is entirely server-decided (never a client-supplied string) - each provider's
// "primary" model is whatever's configured via its NUXT_*_MODEL env var (see nuxt.config.ts) and
// used for 'balanced'/'heavy'; 'light' gets a hardcoded smaller/cheaper model per provider instead
// of adding a NUXT_*_MODEL_LIGHT env var per provider, to keep the .env surface small. Falls back
// to the primary model if a provider has no known lighter option.
const LIGHT_MODEL_OVERRIDE: Partial<Record<AiProviderId, string>> = {
  groq: 'openai/gpt-oss-20b',
  gemini: 'gemini-flash-lite-latest',
  // The only other model on this account's Cerebras catalog besides the primary gpt-oss-120b (see
  // nuxt.config.ts's cerebrasModel comment) - verified live, not guessed.
  cerebras: 'qwen-3.8-27b'
}

export function resolveModel(providerId: AiProviderId, profileId: PromptProfileId, primaryModel: string): string {
  if (profileId === 'light') return LIGHT_MODEL_OVERRIDE[providerId] ?? primaryModel

  return primaryModel
}
