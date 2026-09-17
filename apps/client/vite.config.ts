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

// TitleBar.vue's own h-9 (2.25rem) bar sits above the whole app, and neither Modal nor Slideover
// knows about it - both center/anchor themselves against the full viewport, so their top edge can
// land underneath/behind the bar instead of starting below it.
//
// Slideover's `inset` variant offsets purely from the viewport edges (inset-y-4 = 1rem from the
// very top); forcing its top offset here fixes it. `!` makes it win over inset-y-4's own top
// regardless of Tailwind's generated CSS order; its bottom-4 (and left/right-4) are untouched.
const SLIDEOVER_TOP = 'top-[3.25rem]!'

// Modal's dialog is centered instead, via `top-1/2 left-1/2 -translate-x/y-1/2` (a center *point*,
// not a fixed edge) - so rather than pinning its top like the slideover above, this shifts that
// center point down by half the title bar's height (2.25rem / 2 = 1.125rem), which nudges every
// modal down under the bar while keeping it perfectly centered - vertically, within the remaining
// space below the bar - regardless of how tall its content ends up being.
const MODAL_TOP = 'top-[calc(50%_+_1.125rem)]!'

// https://tauri.app/start/frontend/vite/
export default defineConfig({
  plugins: [
    vue(),
    ui({
      ui: {
        // UButton's own theme only sets `cursor-not-allowed` for its disabled state, leaving the
        // enabled one at the browser's default `cursor: default` for a <button> element (unlike an
        // <a>, which is `pointer` natively) - every button in the app was reading as non-clickable
        // at a glance. Added once here, globally, instead of a per-usage `:ui="{ base: '...' }'"`.
        button: { slots: { base: 'cursor-pointer' } },
        modal: { slots: { overlay: Z_MODAL, content: `${Z_MODAL} ${MODAL_TOP}` } },
        slideover: { slots: { overlay: Z_MODAL, content: `${Z_MODAL} ${SLIDEOVER_TOP}` } },
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
