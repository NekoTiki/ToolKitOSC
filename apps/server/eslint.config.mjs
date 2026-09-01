// @ts-check
import rootConfig from '../../eslint.config.mjs'
import withNuxt from './.nuxt/eslint.config.mjs'

// withNuxt() must stay the base config here — it consumes Nuxt's own generated
// .nuxt/eslint.config.mjs, which only exists for a Nuxt app (not for the shared package
// or the client app). Root's import-sort/prettier-compat rules are layered on top.
export default withNuxt(...rootConfig, {
  rules: {
    '@typescript-eslint/unified-signatures': 'off'
  }
})
