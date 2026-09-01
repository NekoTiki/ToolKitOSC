<script setup lang="ts">
import type { ConnectionState } from '@renderer/components/ConnectionStatus.vue'
import { useClientsListDrawer } from '@renderer/composables/useClientsListDrawer'
import { useOscConnection } from '@renderer/composables/useOscConnection'
import { useWebsocketHost } from '@renderer/composables/useWebsocketHost'
import { computed, onBeforeUnmount, onMounted } from 'vue'

const { clients, status, open, close } = useWebsocketHost()
const { connected: oscConnected } = useOscConnection()
const { openDrawer: openClientsListDrawer } = useClientsListDrawer()

const serverState = computed<ConnectionState>(() =>
  status.value === 'OPEN'
    ? 'connected'
    : status.value === 'CONNECTING'
      ? 'connecting'
      : 'disconnected'
)
const oscState = computed<ConnectionState>(() =>
  oscConnected.value ? 'connected' : 'disconnected'
)

onMounted(open)
onBeforeUnmount(close)
</script>

<template>
  <div class="flex items-center justify-center gap-4">
    <ConnectionStatus icon="i-lucide-server" label="Server" :state="serverState" />
    <ConnectionStatus icon="i-lucide-radio-tower" label="OSC" :state="oscState" />
    <UButton
      icon="solar:users-group-rounded-bold"
      variant="ghost"
      @click="openClientsListDrawer()"
    >
      {{ clients.length }}
    </UButton>
  </div>
</template>

<style scoped></style>
