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

defineSlots<{
  // Extra rows shown under the status line, e.g. the server URL or the connected avatar's name -
  // matches IntifaceStatus.vue's popover treatment instead of this being a plain one-line tooltip.
  // Optional: a consumer with nothing more to say just gets the status line alone.
  default?(): unknown
}>()
</script>

<template>
  <!-- z-20: this renders inside AppHeader.vue's sticky z-10 bar - without its own explicit
  z-index above that, this (like every Nuxt UI overlay, none of which set one by default) would
  render behind it instead of floating over it. -->
  <UPopover
    mode="hover"
    :content="{ align: 'center', side: 'bottom' }"
    :ui="{ content: 'z-20' }"
  >
    <UIcon
      :name="icon"
      class="size-5"
      :class="STATE_COLOR[state]"
    />

    <template #content>
      <div class="flex min-w-52 flex-col gap-2 p-3">
        <div
          class="flex items-center gap-1.5 text-sm font-medium"
          :class="STATE_COLOR[state]"
        >
          <UIcon
            :name="icon"
            class="size-4"
          />
          {{ label }}: {{ STATE_TEXT[state] }}
        </div>

        <slot />
      </div>
    </template>
  </UPopover>
</template>

<style scoped></style>
