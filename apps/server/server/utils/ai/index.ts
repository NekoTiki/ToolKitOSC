import { createGeminiProvider } from './providers/gemini'
import { createGroqProvider } from './providers/groq'
import type { AiProvider } from './types'

// Not re-exported from here - Nitro's server/utils auto-import scanner already picks these up
// directly from types.ts/normalize.ts; re-exporting the same names from this barrel just produces
// duplicate-import warnings for the auto-importer. Import them from '~~/server/utils/ai/types' and
// '~~/server/utils/ai/normalize' directly.

// Built fresh per call from the current runtimeConfig rather than at module scope - keeps this
// testable/hot-reload-friendly and avoids capturing an empty key if this module is ever imported
// before Nuxt has resolved env vars.
export function getAiProviders(): AiProvider[] {
  const config = useRuntimeConfig()

  return [
    createGroqProvider(config.groqApiKey, config.groqModel),
    createGeminiProvider(config.geminiApiKey, config.geminiModel)
  ]
}

export function getAiProvider(id: string): AiProvider | undefined {
  return getAiProviders().find((p) => p.id === id)
}
