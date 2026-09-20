import { isProviderHealthy, pickNextProvider } from './loadBalancer'
import { createCloudflareProvider } from './providers/cloudflare'
import { createGeminiProvider } from './providers/gemini'
import { createGroqProvider } from './providers/groq'
import { createMistralProvider } from './providers/mistral'
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
    createMistralProvider(config.mistralApiKey, config.mistralModel),
    createCloudflareProvider(config.cloudflareApiKey, config.cloudflareAccountId, config.cloudflareModel)
  ]
}

export function getAiProvider(id: string): AiProvider | undefined {
  return getAiProviders().find((p) => p.id === id)
}

// Provider used for accounts without model-select permission (see utils/ai/access.ts) - the server
// decides, never the client. NUXT_AI_DEFAULT_PROVIDER, if set, pins a specific provider as long as
// it's actually usable right now; otherwise every such account spreads across whatever's
// configured via round-robin + a basic circuit breaker (see loadBalancer.ts) instead of all
// landing on one fixed preference order. Returns undefined only if no provider is configured, or
// the pinned one is configured but currently unhealthy with nothing else to fall back to.
export function resolveDefaultProvider(): AiProvider | undefined {
  const providers = getAiProviders().filter((p) => p.isConfigured())
  const preferred = useRuntimeConfig().aiDefaultProvider

  if (preferred) {
    const pinned = providers.find((p) => p.id === preferred)

    if (pinned && isProviderHealthy(pinned.id)) return pinned
  }

  return pickNextProvider(providers)
}
