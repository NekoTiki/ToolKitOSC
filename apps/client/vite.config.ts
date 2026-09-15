import { fileURLToPath, URL } from 'node:url'

import ui from '@nuxt/ui/vite'
import tailwindcss from '@tailwindcss/vite'
import vue from '@vitejs/plugin-vue'
import { defineConfig } from 'vite'

// Every floating/portaled Nuxt UI component (Tooltip, Popover, DropdownMenu, Select, SelectMenu)
// teleports its content to <body> with no z-index of its own (verified against this version's
// theme defaults - none of them set one), and neither does Modal/Slideover. That leaves their
// stacking order to fall out of DOM/mount order, which loses to AppHeader.vue's sticky `z-10`
// bar (an explicit z-index always wins over z-index:auto content in the same stacking context,
// regardless of DOM position) - the exact "renders behind the header" bug this app kept hitting
// one component at a time. Setting it once here, globally, fixes every instance of it - current
// and future - instead of requiring a per-usage `:ui="{ content: 'z-NN' }"` override each time a
// new one is added. The scale:
//   10 - AppHeader.vue's sticky bar (hardcoded there, not part of this config)
//   30 - Modal / Slideover (need to beat the header)
//   40 - Tooltip / Popover / DropdownMenu / Select content (need to beat both the header AND any
//        Modal/Slideover they might be nested inside, e.g. a dropdown inside SettingsModal)
// AreYouSureModal.vue is the one deliberate exception, at z-50: it can itself be triggered from
// inside an already-open Modal/Slideover (e.g. a delete confirmation opened from within
// ClientsListSliderover), so it needs to beat other z-30 modals too, not just the header.
const Z_MODAL = 'z-30'
const Z_OVERLAY = 'z-40'

// https://tauri.app/start/frontend/vite/
export default defineConfig({
  plugins: [
    vue(),
    ui({
      ui: {
        modal: { slots: { overlay: Z_MODAL, content: Z_MODAL } },
        slideover: { slots: { overlay: Z_MODAL, content: Z_MODAL } },
        tooltip: { slots: { content: Z_OVERLAY } },
        popover: { slots: { content: Z_OVERLAY } },
        dropdownMenu: { slots: { content: Z_OVERLAY } },
        select: { slots: { content: Z_OVERLAY } },
        selectMenu: { slots: { content: Z_OVERLAY } },
        inputMenu: { slots: { content: Z_OVERLAY } }
      }
    }),
    tailwindcss()
  ],
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
