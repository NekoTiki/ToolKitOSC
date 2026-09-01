<script setup lang="ts">
export type ConnectionState = 'connected' | 'connecting' | 'disconnected'

defineProps<{
  icon: string
  label: string
  state: ConnectionState
}>()

const STATE_COLOR: Record<ConnectionState, string> = {
  connected: 'text-success',
  connecting: 'text-warning',
  disconnected: 'text-error'
}

const STATE_TEXT: Record<ConnectionState, string> = {
  connected: 'Connected',
  connecting: 'Connecting…',
  disconnected: 'Disconnected'
}
</script>

<template>
  <UTooltip>
    <UIcon :name="icon" class="size-5" :class="STATE_COLOR[state]" />

    <template #content> {{ label }}: {{ STATE_TEXT[state] }} </template>
  </UTooltip>
</template>

<style scoped></style>
