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
  <!-- z-[60]: App.vue mounts this before ClientsListSliderover, so at equal z-50 the slideover
  (e.g. the ban confirmation triggered from ClientDetails.vue) would paint on top of this modal -
  same reasoning as the z-[60] popovers in ClientDetails.vue. -->
  <UModal
    v-model:open="open"
    class="z-[60]"
    :ui="{ overlay: 'z-[60]', header: 'p-4 sm:p-4 text-lg', body: 'p-4 sm:p-4' }"
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
