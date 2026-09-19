import { createCerebrasProvider } from './providers/cerebras'
import { createCloudflareProvider } from './providers/cloudflare'
import { createGeminiProvider } from './providers/gemini'
import { createGroqProvider } from './providers/groq'
import { createOpenRouterProvider } from './providers/openrouter'
import type { AiProvider } from './types'

// Not re-exported from here - Nitro's server/utils auto-import scanner already picks these up
// directly from types.ts/normalize.ts/profiles.ts; re-exporting the same names from this barrel
// just produces duplicate-import warnings for the auto-importer. Import them from
// '~~/server/utils/ai/types', '~~/server/utils/ai/normalize', '~~/server/utils/ai/profiles' etc.
// directly.

// Built fresh per call from the current runtimeConfig rather than at module scope - keeps this
// testable/hot-reload-friendly and avoids capturing an empty key if this module is ever imported
// before Nuxt has resolved env vars.
export function getAiProviders(): AiProvider[] {
  const config = useRuntimeConfig()

  return [
    createGroqProvider(config.groqApiKey, config.groqModel),
    createGeminiProvider(config.geminiApiKey, config.geminiModel),
    createOpenRouterProvider(config.openrouterApiKey, config.openrouterModel),
    createCerebrasProvider(config.cerebrasApiKey, config.cerebrasModel),
    createCloudflareProvider(config.cloudflareApiKey, config.cloudflareAccountId, config.cloudflareModel)
  ]
}

export function getAiProvider(id: string): AiProvider | undefined {
  return getAiProviders().find((p) => p.id === id)
}

// Fixed fallback order when NUXT_AI_DEFAULT_PROVIDER is unset or points at a provider that isn't
// actually configured - gemini first since it was the cheapest/most reliable in this feature's own
// testing (see nuxt.config.ts's geminiModel comment).
const DEFAULT_PROVIDER_PREFERENCE = ['gemini', 'groq', 'openrouter', 'cerebras', 'cloudflare']

// Provider used for accounts without model-select permission (see utils/ai/access.ts) - the server
// decides, never the client. Returns undefined only if no provider is configured at all.
export function resolveDefaultProvider(): AiProvider | undefined {
  const providers = getAiProviders()
  const preferred = useRuntimeConfig().aiDefaultProvider

  const preferredMatch = preferred && providers.find((p) => p.id === preferred && p.isConfigured())

  if (preferredMatch) return preferredMatch

  for (const id of DEFAULT_PROVIDER_PREFERENCE) {
    const match = providers.find((p) => p.id === id && p.isConfigured())

    if (match) return match
  }

  return providers.find((p) => p.isConfigured())
}
