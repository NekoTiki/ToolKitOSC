<script setup lang="ts">
import { computed } from 'vue'

export type ConnectionState = 'connected' | 'connecting' | 'disconnected'

const props = defineProps<{
  icon: string
  label: string
  state: ConnectionState
  // Flags a connection as degraded without changing `state` itself - e.g. the desktop client is
  // talking to the server fine, but the server can't render one of its control types yet (see
  // StatusBar.vue's use of `unsupportedControlTypes`). Only overrides the color, not the label -
  // the connection genuinely is 'connected', just not fully.
  warning?: boolean
  // The shared anchor every header status popover positions against (see StatusBar.vue) - lets
  // them all open with the same left edge/baseline instead of each one centering under its own
  // icon, which sits at a different x position for each.
  reference?: HTMLElement | null
}>()

const STATE_COLOR: Record<ConnectionState, string> = {
  connected: 'text-success',
  connecting: 'text-warning',
  disconnected: 'text-error'
}

const iconColor = computed(() => (props.warning ? 'text-warning' : STATE_COLOR[props.state]))

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
  <!-- No z-index override needed here - the popover's z-index is set globally, above
  AppHeader.vue's sticky bar, in vite.config.ts. -->
  <UPopover
    mode="hover"
    :reference="reference ?? undefined"
    :content="{ align: 'start', side: 'bottom' }"
  >
    <UIcon
      :name="icon"
      class="size-5"
      :class="iconColor"
    />

    <template #content>
      <div class="flex max-w-72 min-w-52 flex-col gap-2 p-3">
        <div
          class="flex items-center gap-1.5 text-sm font-medium"
          :class="iconColor"
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
