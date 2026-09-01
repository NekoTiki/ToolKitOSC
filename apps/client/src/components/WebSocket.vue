<script setup lang="ts">
import type { ConnectionState } from '@renderer/components/ConnectionStatus.vue'
import { useOscConnection } from '@renderer/composables/useOscConnection'
import { getGuestAvatar, useWebsocketHost } from '@renderer/composables/useWebsocketHost'
import { computed, onBeforeUnmount, onMounted } from 'vue'

const { clients, status, open, close } = useWebsocketHost()
const { connected: oscConnected } = useOscConnection()

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
    <UPopover mode="click" :content="{ align: 'center', side: 'bottom', sideOffset: 8 }">
      <UButton icon="solar:users-group-rounded-bold" variant="ghost">
        {{ clients.length }}
      </UButton>

      <template #content>
        <UCard :ui="{ body: 'p-2 sm:p-2 space-y-2' }">
          <ClientDetails
            v-for="(client, index) in clients"
            :key="index"
            :client="{
              avatar: client.user?.discord
                ? client.user.discord?.avatar
                : getGuestAvatar(client.peerId, client.user?.userDefinedDisplayName || 'Guest'),
              displayName:
                client.user?.discord?.name || client.user?.userDefinedDisplayName || 'Guest',
              ip: client.ip,
              discordId: client.user?.discord ? client.user.discord?.id : null
            }"
          />
        </UCard>
      </template>
    </UPopover>
  </div>
</template>

<style scoped></style>
