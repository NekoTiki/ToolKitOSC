import { fileURLToPath, URL } from 'node:url'

import ui from '@nuxt/ui/vite'
import tailwindcss from '@tailwindcss/vite'
import vue from '@vitejs/plugin-vue'
import { defineConfig } from 'vite'

// https://tauri.app/start/frontend/vite/
export default defineConfig({
  plugins: [vue(), ui(), tailwindcss()],
  resolve: {
    alias: {
      '@renderer': fileURLToPath(new URL('./src', import.meta.url))
    }
  },
  // Tauri expects a fixed, predictable dev server port and needs its own error overlay instead
  // of Vite's screen-clearing behavior stepping on Rust build output.
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
    watch: {
      // don't watch `src-tauri` — Rust's own file-watcher (`tauri dev`) already triggers a
      // rebuild+relaunch on Rust changes; Vite reacting too just causes a redundant reload.
      ignored: ['**/src-tauri/**']
    }
  },
  envPrefix: ['VITE_', 'TAURI_ENV_*'],
  build: {
    // Tauri uses Chromium on Windows/Linux and WebKit on macOS.
    target: process.env.TAURI_ENV_PLATFORM === 'windows' ? 'chrome105' : 'safari13',
    minify: !process.env.TAURI_ENV_DEBUG ? 'esbuild' : false,
    sourcemap: !!process.env.TAURI_ENV_DEBUG
  }
})
