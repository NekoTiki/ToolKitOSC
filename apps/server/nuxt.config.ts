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
    }
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
