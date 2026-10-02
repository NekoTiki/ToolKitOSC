<script setup lang="ts">
import ConfirmDialog from '@renderer/components/ui/ConfirmDialog.vue'
import { computed } from 'vue'

const props = defineProps<{
  title: string
  message: string
  confirmText: string
  cancelText: string
  danger: boolean
}>()

// Resolves useAreYouSureModal.ts's returned promise - explicit on both buttons so it resolves the
// instant one is clicked, not after the close transition finishes; @after:leave below is only the
// fallback for every other way to dismiss (Escape, backdrop click) - useOverlay's own close() is a
// no-op once the promise has already resolved once, so there's no risk of a button click's
// `true`/`false` getting overwritten by the leave-transition's own `false`.
const emit = defineEmits<{ close: [boolean] }>()
const open = defineModel<boolean>('open')

// `message` may contain **bold** markers to call out important bits (an IP, a display name, a ban
// scope, ...). Split into text/bold segments and interpolate as plain text nodes (never v-html) -
// message content can ultimately trace back to a remote client's own self-chosen display name, so
// it must never be parsed as HTML.
const messageParts = computed<{ text: string; bold: boolean }[]>(() =>
  props.message
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
  <ConfirmDialog
    v-model:open="open"
    :title="title"
    :confirm-text="confirmText"
    :cancel-text="cancelText"
    :danger="danger"
    @confirm="emit('close', true)"
    @cancel="emit('close', false)"
    @after:leave="emit('close', false)"
  >
    <p>
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
  </ConfirmDialog>
</template>
