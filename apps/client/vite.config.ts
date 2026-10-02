import { fileURLToPath, URL } from 'node:url'

import ui from '@nuxt/ui/vite'
import tailwindcss from '@tailwindcss/vite'
import vue from '@vitejs/plugin-vue'
import { defineConfig } from 'vite'

// Every floating/portaled Nuxt UI component (Tooltip, Popover, DropdownMenu, Select, SelectMenu,
// ContextMenu) teleports its content to <body> with no z-index of its own (verified against this
// version's theme defaults - none of them set one), and neither does Modal/Slideover. That leaves
// their stacking order to fall out of DOM/mount order, which loses to any in-page content with an
// explicit z-index (an explicit z-index always wins over z-index:auto content in the same stacking
// context, regardless of DOM position) - the exact "renders behind the header" bug this app kept
// hitting one component at a time. Setting it once here, globally, fixes every instance of it -
// current and future - instead of requiring a per-usage `:ui="{ content: 'z-NN' }"` override each
// time a new one is added. The scale:
//   <30 - in-page positioned content (sticky page parts, tile edit overlays)
//   30 - Modal / Slideover (need to beat in-page content)
//   40 - Tooltip / Popover / DropdownMenu / Select / ContextMenu content (need to beat both
//        in-page content AND any Modal/Slideover they might be nested inside, e.g. a dropdown
//        inside SettingsModal)
// Confirmation dialogs (ui/ConfirmDialog.vue) are the one deliberate exception, at z-50: they can
// be triggered from inside something already open (e.g. a delete confirmation opened from a
// context menu), so they need to beat other z-30 modals too.
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
      // Aurora is dark-only (see shared-ui's styles/aurora.css) - index.html carries a static
      // `dark` class, and this stops Nuxt UI's color-mode helper from swapping it for `light` on
      // systems set to light mode.
      colorMode: false,
      ui: {
        // The defaults for a user who hasn't picked a theme yet (useTheme.ts only overrides these
        // once a choice is stored). Same pair as the server's app.config.ts, so a viewer sees the
        // same colors before the host's own theme arrives.
        colors: { primary: 'teal', secondary: 'orange', neutral: 'slate' },
        // Larger defaults across the board: this app is often used from inside VR with a laser
        // pointer, where the stock `md` controls are too small to hit reliably.
        input: { defaultVariants: { size: 'lg' } },
        // Centered, not top-aligned: next to a label + description the toggle otherwise sits high.
        switch: { slots: { root: 'relative flex items-center' }, defaultVariants: { size: 'lg' } },
        checkbox: { defaultVariants: { size: 'lg' } },
        // UButton's own theme only sets `cursor-not-allowed` for its disabled state, leaving the
        // enabled one at the browser's default `cursor: default` for a <button> element (unlike an
        // <a>, which is `pointer` natively) - every button in the app was reading as non-clickable
        // at a glance. Added once here, globally, instead of a per-usage `:ui="{ base: '...' }'"`.
        button: { slots: { base: 'cursor-pointer' }, defaultVariants: { size: 'lg' } },
        modal: { slots: { overlay: Z_MODAL, content: `${Z_MODAL} ${MODAL_TOP}` } },
        slideover: { slots: { overlay: Z_MODAL, content: `${Z_MODAL} ${SLIDEOVER_TOP}` } },
        tooltip: { slots: { content: Z_OVERLAY } },
        popover: { slots: { content: Z_OVERLAY } },
        dropdownMenu: { slots: { content: Z_OVERLAY } },
        select: { slots: { content: Z_OVERLAY }, defaultVariants: { size: 'lg' } },
        selectMenu: { slots: { content: Z_OVERLAY }, defaultVariants: { size: 'lg' } },
        inputMenu: { slots: { content: Z_OVERLAY }, defaultVariants: { size: 'lg' } },
        contextMenu: { slots: { content: Z_OVERLAY } }
      }
    }),
    tailwindcss()
  ],
  resolve: {
    // vue-router isn't hoisted to the workspace root (client and server pin different majors), so
    // @nuxt/ui - which is hoisted - can't resolve it from its own location and falls back to an
    // empty optional-peer-dep stub, failing the production build on UButton's RouterLink import.
    // Deduping resolves it from this package instead.
    dedupe: ['vue', 'vue-router'],
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
    // Tauri uses WebView2 (Chromium) on Windows and WebKit elsewhere (WebKitGTK on Linux). safari13
    // is too old for esbuild to lower some of the destructuring in our deps, and any WebKitGTK
    // 4.1 shipped by current distros (Ubuntu 22.04+) is past Safari 16 level.
    target: process.env.TAURI_ENV_PLATFORM === 'windows' ? 'chrome105' : 'safari16',
    minify: !process.env.TAURI_ENV_DEBUG ? 'esbuild' : false,
    sourcemap: !!process.env.TAURI_ENV_DEBUG
  }
})
