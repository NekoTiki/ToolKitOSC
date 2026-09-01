import { defineConfig } from 'eslint/config'
import tailwind from 'eslint-plugin-tailwindcss'
import eslintPluginVue from 'eslint-plugin-vue'
import tseslint from 'typescript-eslint'
import vueParser from 'vue-eslint-parser'

import rootConfig from '../../eslint.config.mjs'

export default defineConfig(
  rootConfig,
  tseslint.configs.recommended,
  eslintPluginVue.configs['flat/recommended'],
  tailwind.configs['flat/recommended'] || tailwind.configs.recommended,
  {
    settings: {
      tailwindcss: {
        cssConfigPath: './src/assets/main.css'
      }
    }
  },
  {
    files: ['**/*.vue'],
    languageOptions: {
      parser: vueParser,
      parserOptions: {
        ecmaFeatures: {
          jsx: true
        },
        extraFileExtensions: ['.vue'],
        parser: tseslint.parser
      }
    }
  },
  {
    files: ['**/*.{ts,mts,tsx,vue}'],
    rules: {
      '@typescript-eslint/consistent-type-imports': 'error',
      'vue/require-default-prop': 'off',
      'vue/multi-word-component-names': 'off',
      'vue/block-lang': [
        'error',
        {
          script: {
            lang: 'ts'
          }
        }
      ]
    }
  }
)
