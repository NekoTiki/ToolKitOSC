// https://nuxt.com/docs/api/configuration/nuxt-config
export default defineNuxtConfig({
  modules: ['nuxt-auth-utils', '@nuxt/ui', '@vueuse/nuxt', '@nuxt/eslint'],
  devtools: { enabled: true },
  css: ['~/assets/css/main.css'],
  // Aurora (shared-ui's styles/aurora.css) is dark-only - pin the color mode rather than following
  // the visitor's OS setting, so the share page always matches the host's desktop client.
  colorMode: { preference: 'dark', fallback: 'dark' },
  // /profile was renamed /connect to match its menu item; old links and bookmarks still work.
  routeRules: { '/profile': { redirect: '/connect' } },
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
    mistralApiKey: '',
    // mistral-medium-latest/mistral-small-latest both 429 with a hard 0 req/minute quota on a
    // free-tier key (verified live against this account) - ministral-14b-latest is the largest
    // model that's actually usable without a paid workspace, and still supports strict
    // json_schema mode. Reverify against GET https://api.mistral.ai/v1/models if upgrading plans.
    mistralModel: 'ministral-14b-latest',
    cloudflareApiKey: '',
    // Workers AI is scoped under a Cloudflare account, not just an API token - both are required
    // for this provider to report itself configured (see providers/cloudflare.ts).
    cloudflareAccountId: '',
    // '@cf/meta/llama-3.1-8b-instruct' initially seemed reasonable but turned out to route to a
    // deprecated internal model ("infire-llama-3.1-8b-instruct") and 410'd - verified against the
    // account's live model catalog (GET /accounts/{id}/ai/models/search) instead of guessing
    // again; gpt-oss-120b is also already the default elsewhere (Groq, OpenRouter).
    cloudflareModel: '@cf/openai/gpt-oss-120b',
    // Invite-only gate for the whole AI feature (see server/utils/ai/access.ts) - comma-separated
    // Discord ids, checked by isAdmin.ts. Admins implicitly have full AI access + model-select
    // permission on top of being able to manage the allowlist from /dashboard.
    aiAdminDiscordIds: '',
    // Provider/profile used for every generation from an account without model-select permission
    // (see profiles.ts's resolveModel and the admin-managed `can_select_model` flag) - falls back
    // to the first configured provider in a fixed preference order if left unset.
    aiDefaultProvider: '',
    aiDefaultProfile: 'balanced',
    // Where the sqlite file (drizzle + node:sqlite, see server/db/index.ts) lives - relative to
    // Nitro's cwd (apps/server in dev, /app in the production Docker image, where docker-compose.yml
    // mounts a host volume at this same path so it survives image rebuilds).
    sqlitePath: './data/app.db'
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
  // @toolkitosc/shared-ui ships raw, uncompiled .vue/.ts source (workspace package, no
  // build step) — Nitro/Vite need to run their own SFC compiler over it rather than treating it
  // as pre-built JS.
  build: {
    transpile: ['@toolkitosc/shared-ui']
  }
})
