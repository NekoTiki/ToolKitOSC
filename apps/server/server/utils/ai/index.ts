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
