<script setup lang="ts">
import logoUrl from '@renderer/assets/logo.svg'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { onMounted, onUnmounted, ref } from 'vue'

const appWindow = getCurrentWindow()
const isMaximized = ref(false)

const refreshMaximized = async (): Promise<void> => {
  isMaximized.value = await appWindow.isMaximized()
}

let unlisten: (() => void) | undefined

onMounted(async () => {
  await refreshMaximized()
  // No dedicated "maximize changed" event exists - a resize is fired on every maximize/restore
  // (and only there, since this window isn't otherwise resized programmatically), so it doubles
  // as the signal to re-check.
  unlisten = await appWindow.onResized(() => void refreshMaximized())
})

onUnmounted(() => unlisten?.())

const minimize = (): void => void appWindow.minimize()
const toggleMaximize = (): void => void appWindow.toggleMaximize()
const close = (): void => void appWindow.close()
</script>

<template>
  <!-- data-tauri-drag-region: Tauri's webview intercepts a press-and-drag anywhere this attribute
  covers and moves the window with it (needs core:window:allow-start-dragging, see
  capabilities/default.json) - this is what replaces the native title bar's own drag behavior. It
  deliberately only wraps the logo/title section (flex-1, so it still fills all the empty space to
  the buttons' left) rather than the whole bar - putting it on a shared ancestor of the window
  control buttons risks the drag starting on their mousedown before a click can register. -->
  <!-- relative z-50: modals/slideovers (UModal/USlideover) are teleported to the end of <body> with
  no explicit z-index of their own, so they'd otherwise paint over this in DOM order. A positive
  z-index here outranks their z-index:auto stacking, keeping the window controls and drag region
  clickable even while a modal is open.
  pointer-events-auto: a modal dialog sets `document.body.style.pointerEvents = 'none'` while open
  (reka-ui's disableOutsidePointerEvents) and only re-enables it on the dialog content itself -
  without overriding that inherited 'none' back to 'auto' here, this bar wouldn't even be a valid
  click target while a modal is open, no matter its z-index.
  @pointerdown.stop: reka-ui's Dialog treats any pointerdown that bubbles all the way up to
  `document` as a click "outside" the modal and closes it - it doesn't matter that it visually hit
  this bar rather than the overlay. Stopping propagation here keeps that pointerdown from ever
  reaching `document`, without affecting the buttons' own @click handlers below. -->
  <div
    class="pointer-events-auto relative z-50 flex h-9 shrink-0 items-center border-b border-default select-none"
    @pointerdown.stop
  >
    <div
      data-tauri-drag-region
      class="flex flex-1 items-center gap-2 self-stretch pl-3 text-xs font-medium text-muted"
      @dblclick="toggleMaximize"
    >
      <img
        :src="logoUrl"
        alt=""
        class="size-4 rounded-xs"
      >
      <span>VRC OSC Toolkit</span>
    </div>

    <div class="flex h-full">
      <button
        type="button"
        aria-label="Minimize"
        class="flex h-full w-11 items-center justify-center text-muted transition-colors hover:bg-elevated hover:text-default"
        @click="minimize"
      >
        <UIcon
          name="i-lucide-minus"
          class="size-4"
        />
      </button>
      <button
        type="button"
        :aria-label="isMaximized ? 'Restore' : 'Maximize'"
        class="flex h-full w-11 items-center justify-center text-muted transition-colors hover:bg-elevated hover:text-default"
        @click="toggleMaximize"
      >
        <UIcon
          :name="isMaximized ? 'i-lucide-copy' : 'i-lucide-square'"
          class="size-3.5"
        />
      </button>
      <button
        type="button"
        aria-label="Close"
        class="flex h-full w-11 items-center justify-center text-muted transition-colors hover:bg-error hover:text-white"
        @click="close"
      >
        <UIcon
          name="i-lucide-x"
          class="size-4"
        />
      </button>
    </div>
  </div>
</template>

<style scoped></style>
