// https://nuxt.com/docs/api/configuration/nuxt-config
export default defineNuxtConfig({
  modules: ['nuxt-auth-utils', '@nuxt/ui', '@vueuse/nuxt', '@nuxt/eslint'],
  devtools: { enabled: true },
  css: ['~/assets/css/main.css'],
  app: {
    head: {
      link: [
        { rel: 'icon', type: 'image/svg+xml', href: '/favicon.svg' },
        { rel: 'shortcut icon', href: '/favicon.ico' }
      ]
    }
  },
  runtimeConfig: {
    session: {
      password: '',
      maxAge: 60 * 60 * 24 * 30
    },
    // AI control-suggestion feature (see server/utils/ai) - private (server-only), never exposed
    // to the client. One key/model pair per supported provider; a provider with no key set is
    // simply reported unavailable by listProviders() rather than the app failing to boot.
    groqApiKey: '',
    groqModel: 'openai/gpt-oss-120b',
    geminiApiKey: '',
    // The full '-flash' tier returned transient 503 'high demand' errors in testing against a
    // fresh key while the lite tier answered reliably (and its free-tier quota is more generous
    // anyway - see the free-tier comparison this feature was scoped against) - kept as the default
    // rather than 'gemini-flash-latest' for that reason, override via NUXT_GEMINI_MODEL if needed.
    geminiModel: 'gemini-flash-lite-latest',
    openrouterApiKey: '',
    openrouterModel: 'openai/gpt-oss-120b',
    cerebrasApiKey: '',
    cerebrasModel: 'llama-3.3-70b',
    cloudflareApiKey: '',
    // Workers AI is scoped under a Cloudflare account, not just an API token - both are required
    // for this provider to report itself configured (see providers/cloudflare.ts).
    cloudflareAccountId: '',
    // '@cf/meta/llama-3.1-8b-instruct' initially seemed reasonable but turned out to route to a
    // deprecated internal model ("infire-llama-3.1-8b-instruct") and 410'd - verified against the
    // account's live model catalog (GET /accounts/{id}/ai/models/search) instead of guessing
    // again; gpt-oss-120b is also already the default elsewhere (Groq, OpenRouter).
    cloudflareModel: '@cf/openai/gpt-oss-120b'
  },
  devServer: {
    port: 3000,
    host: '0.0.0.0'
  },
  compatibilityDate: '2025-07-15',
  nitro: {
    experimental: {
      websocket: true
    }
  },
  // @vrc-osc-toolkit/shared-ui ships raw, uncompiled .vue/.ts source (workspace package, no
  // build step) — Nitro/Vite need to run their own SFC compiler over it rather than treating it
  // as pre-built JS.
  build: {
    transpile: ['@vrc-osc-toolkit/shared-ui']
  }
})
