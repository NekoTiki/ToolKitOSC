// Lightweight lint config for the shared package: vue + ts recommended rules on top of the
// root config. Deliberately no tailwindcss-plugin linting and no Nuxt-specific rules here —
// this package doesn't own a Tailwind config or a Nuxt app, and its consumers (apps/client,
// apps/server) already lint class-name usage on their own side.
import { defineConfig } from 'eslint/config'
import eslintPluginVue from 'eslint-plugin-vue'
import tseslint from 'typescript-eslint'
import vueParser from 'vue-eslint-parser'

import rootConfig from '../../eslint.config.mjs'

export default defineConfig(
  rootConfig,
  tseslint.configs.recommended,
  eslintPluginVue.configs['flat/recommended'],
  {
    files: ['**/*.vue'],
    languageOptions: {
      parser: vueParser,
      parserOptions: {
        extraFileExtensions: ['.vue'],
        parser: tseslint.parser
      }
    }
  },
  {
    files: ['**/*.{ts,vue}'],
    rules: {
      '@typescript-eslint/consistent-type-imports': 'error',
      'vue/require-default-prop': 'off',
      'vue/multi-word-component-names': 'off'
    }
  }
)
