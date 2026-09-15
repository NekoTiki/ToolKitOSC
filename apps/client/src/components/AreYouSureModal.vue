<script setup lang="ts">
import { useAreYouSureModal } from '@renderer/composables/useAreYouSureModal'
import { computed } from 'vue'

const { open, options, cancel, confirm } = useAreYouSureModal()

// `options.message` may contain **bold** markers to call out important bits (an IP, a display
// name, a ban scope, ...). Split into text/bold segments and interpolate as plain text nodes
// (never v-html) - message content can ultimately trace back to a remote client's own
// self-chosen display name, so it must never be parsed as HTML.
const messageParts = computed<{ text: string; bold: boolean }[]>(() =>
  options.value.message
    .split(/(\*\*[^*]+\*\*)/g)
    .filter(Boolean)
    .map((part) =>
      part.startsWith('**') && part.endsWith('**')
        ? { text: part.slice(2, -2), bold: true }
        : { text: part, bold: false }
    )
)
</script>

<template>
  <!-- Every Modal/Slideover in this app defaults to the same z-30 (set globally in
  vite.config.ts), which is normally fine since they don't overlap - but this one is a
  confirmation dialog that's routinely opened from INSIDE another already-open one (e.g. the ban
  confirmation triggered from ClientDetails.vue, itself inside ClientsListSliderover). Two equal
  z-30 overlays would fall back to DOM/mount order (App.vue mounts this one first, so it would
  lose), so this is deliberately bumped above that shared tier - and above the z-40 tooltip/
  popover/dropdown/select tier too, so it wins even if something like that is still open. -->
  <UModal
    v-model:open="open"
    class="z-50"
    :ui="{ overlay: 'z-50', header: 'p-4 sm:p-4 text-lg', body: 'p-4 sm:p-4' }"
    @after:leave="cancel"
  >
    <template #header>
      {{ options.title }}
    </template>
    <template #body>
      <p class="wrap-break-word whitespace-pre-wrap">
        <template
          v-for="(part, index) in messageParts"
          :key="index"
        >
          <strong v-if="part.bold">{{ part.text }}</strong>
          <template v-else>
            {{ part.text }}
          </template>
        </template>
      </p>
    </template>
    <template #footer>
      <div class="flex w-full justify-end gap-2">
        <UButton
          color="neutral"
          variant="subtle"
          @click="cancel"
        >
          {{ options.cancelText }}
        </UButton>
        <UButton
          color="error"
          @click="confirm"
        >
          {{ options.confirmText }}
        </UButton>
      </div>
    </template>
  </UModal>
</template>

<style scoped></style>
