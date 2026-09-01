// Root ESLint config: universal rules only (ignores + import sort + prettier compat).
// apps/* and packages/* each have their own eslint.config.mjs that imports this as a base
// and layers framework-specific config (Nuxt, Electron/Tauri, Tailwind linting, ...) on top.
import eslintConfigPrettier from 'eslint-config-prettier'
import { defineConfig } from 'eslint/config'
import simpleImportSort from 'eslint-plugin-simple-import-sort'

export default defineConfig(
  {
    ignores: [
      '**/node_modules',
      '**/dist',
      '**/out',
      '**/.nuxt',
      '**/.output',
      '**/src-tauri/target',
      '**/src-tauri/gen'
    ]
  },
  {
    plugins: {
      'simple-import-sort': simpleImportSort
    },
    rules: {
      'simple-import-sort/imports': 'error',
      'simple-import-sort/exports': 'error'
    }
  },
  eslintConfigPrettier
)
