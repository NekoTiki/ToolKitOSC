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
  <div class="flex h-9 shrink-0 items-center border-b border-default bg-default select-none">
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
