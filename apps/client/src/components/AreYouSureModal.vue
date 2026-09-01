<script setup lang="ts">
import { useAreYouSureModal } from '@renderer/composables/useAreYouSureModal'

const { open, options, cancel, confirm } = useAreYouSureModal()
</script>

<template>
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
      <p class="whitespace-pre-wrap">
        {{ options.message }}
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
