<script setup lang="ts">
import type { CreatedBy } from '@toolkitosc/shared-ui'

// Shown in an editor while what it edits is an AI assistant's draft (src-tauri/src/mcp): Keep
// makes it yours, as does saving a change; Discard deletes it.
defineProps<{ createdBy: CreatedBy }>()

const emit = defineEmits<{ keep: []; discard: [] }>()
</script>

<template>
  <div class="flex w-full basis-full flex-wrap items-center gap-3 rounded-2xl border border-primary/40 bg-primary/10 px-4 py-3">
    <UIcon
      name="i-lucide-bot"
      class="size-5 shrink-0 text-primary"
    />
    <p class="min-w-0 flex-1 text-sm text-default">
      A draft from <b class="text-highlighted">{{ createdBy.assistant }}</b>. Look it over, then keep it or throw it away.
    </p>
    <UButton
      label="Keep"
      icon="i-lucide-check"
      @click="emit('keep')"
    />
    <UButton
      label="Discard"
      icon="i-lucide-trash-2"
      color="error"
      variant="soft"
      @click="emit('discard')"
    />
  </div>
</template>
