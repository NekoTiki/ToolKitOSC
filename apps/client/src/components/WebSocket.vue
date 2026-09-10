<script setup lang="ts">
import type { ConnectionState } from '@renderer/components/ConnectionStatus.vue'
import { useAvatarDetails } from '@renderer/composables/useAvatarDetails'
import { useClientsListDrawer } from '@renderer/composables/useClientsListDrawer'
import { useOscConnection } from '@renderer/composables/useOscConnection'
import { useWebsocketHost } from '@renderer/composables/useWebsocketHost'
import { useWebsocketSettings } from '@renderer/composables/useWebsocketSettings'
import { computed, onBeforeUnmount, onMounted } from 'vue'

const { clients, status, open, close } = useWebsocketHost()
const { connected: oscConnected } = useOscConnection()
const { avatarDetails } = useAvatarDetails()
const { serverWsUrl } = useWebsocketSettings()
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
    <ConnectionStatus
      icon="i-lucide-server"
      label="Server"
      :state="serverState"
    >
      <div class="flex items-center justify-between gap-3 text-sm">
        <span class="text-muted">URL</span>
        <span class="truncate">{{ serverWsUrl }}</span>
      </div>
      <div class="flex items-center justify-between gap-3 text-sm">
        <span class="text-muted">Clients</span>
        <span>{{ clients.length }}</span>
      </div>
    </ConnectionStatus>
    <ConnectionStatus
      icon="i-lucide-radio-tower"
      label="OSC"
      :state="oscState"
    >
      <div
        v-if="avatarDetails"
        class="flex items-center justify-between gap-3 text-sm"
      >
        <span class="text-muted">Avatar</span>
        <span class="truncate">{{ avatarDetails.name }}</span>
      </div>
      <div
        v-else
        class="text-xs text-muted"
      >
        No avatar detected yet
      </div>
    </ConnectionStatus>
    <IntifaceStatus />
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
