<script setup lang="ts">
const props = defineProps<{
  scope: 'ip' | 'discord' | null
  reason?: string | null
}>()

const open = defineModel<boolean>('open')

const modeLabel = computed(() => (props.scope === 'discord' ? 'Discord account' : 'IP address'))
</script>

<template>
  <UModal v-model:open="open" title="Access Denied" class="w-[400px]">
    <template #body>
      <div class="flex flex-col items-center gap-3 text-center">
        <UIcon name="i-lucide-ban" class="text-error size-10" />
        <p class="whitespace-pre-wrap break-words">
          You've been banned by your <strong>{{ modeLabel }}</strong> and can no longer use this
          control.
        </p>
        <p v-if="reason" class="whitespace-pre-wrap break-words text-sm text-muted">
          <strong>Reason:</strong> {{ reason }}
        </p>
      </div>
    </template>
  </UModal>
</template>

<style scoped></style>
