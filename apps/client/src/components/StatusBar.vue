<script setup lang="ts">
import type { ConnectionState } from '@renderer/components/ConnectionStatus.vue'
import { useAvatarDetails } from '@renderer/composables/useAvatarDetails'
import { useClientsListDrawer } from '@renderer/composables/useClientsListDrawer'
import { useOscConnection } from '@renderer/composables/useOscConnection'
import {
  protocolMismatchReason,
  serverProtocolVersion,
  unsupportedControlTypes,
  useWebsocketHost
} from '@renderer/composables/useWebsocketHost'
import { useWebsocketSettings } from '@renderer/composables/useWebsocketSettings'
import { CONTROL_TYPE_LABELS, PROTOCOL_VERSION } from '@vrc-osc-toolkit/shared-ui'
import { computed, onBeforeUnmount, onMounted } from 'vue'

defineProps<{
  // The header bar element every status popover below positions against (see AppHeader.vue) - lets
  // them all open flush with its left edge, right below it, instead of each one centering under
  // its own icon, which sits at a different x position for each.
  reference?: HTMLElement | null
}>()

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
// A real mismatch (protocolMismatchReason) is the authoritative "incompatible" signal - the
// server itself rejected this client. Falling back to a plain version-number diff covers the
// (currently impossible, but future) case of a compat shim letting a differing version through
// without ever sending protocol-mismatch.
const protocolCompatible = computed<boolean>(() => {
  if (protocolMismatchReason.value) return false
  if (serverProtocolVersion.value === null) return true

  return serverProtocolVersion.value === PROTOCOL_VERSION
})

const unsupportedControlTypeLabels = computed<string>(() =>
  unsupportedControlTypes.value.map((type) => CONTROL_TYPE_LABELS[type]).join(', ')
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
      :warning="unsupportedControlTypes.length > 0"
      :reference="reference"
    >
      <div class="flex items-center justify-between gap-3 text-sm">
        <span class="text-muted">URL</span>
        <span class="truncate">{{ serverWsUrl }}</span>
      </div>
      <div class="flex items-center justify-between gap-3 text-sm">
        <span class="text-muted">Clients</span>
        <span>{{ clients.length }}</span>
      </div>
      <div class="flex items-center justify-between gap-3 text-sm">
        <span class="text-muted">Protocol</span>
        <span
          class="flex items-center gap-1"
          :class="{ 'font-medium text-error': !protocolCompatible }"
        >
          <UIcon
            v-if="!protocolCompatible"
            name="i-lucide-triangle-alert"
            class="size-3.5"
          />
          Client v{{ PROTOCOL_VERSION }}<template v-if="serverProtocolVersion !== null">
            / Server v{{ serverProtocolVersion }}</template>
        </span>
      </div>
      <div
        v-if="protocolMismatchReason"
        class="text-xs text-error"
      >
        {{ protocolMismatchReason }}
      </div>
      <div
        v-if="unsupportedControlTypes.length > 0"
        class="flex items-start gap-1.5 text-xs text-warning"
      >
        <UIcon
          name="i-lucide-triangle-alert"
          class="mt-0.5 size-3.5 shrink-0"
        />
        <span>
          This server can't display these control types yet, it should be updated:
          {{ unsupportedControlTypeLabels }}
        </span>
      </div>
    </ConnectionStatus>
    <ConnectionStatus
      icon="i-lucide-radio-tower"
      label="OSC"
      :state="oscState"
      :reference="reference"
    >
      <template v-if="avatarDetails">
        <div class="flex items-center justify-between gap-3 text-sm">
          <span class="text-muted">Name</span>
          <span class="truncate">{{ avatarDetails.name }}</span>
        </div>
        <div class="flex items-center justify-between gap-3 text-sm">
          <span class="text-muted">ID</span>
          <span class="truncate">{{ avatarDetails.id }}</span>
        </div>
        <div class="flex items-center justify-between gap-3 text-sm">
          <span class="text-muted">Hash</span>
          <span class="truncate">{{ avatarDetails.hash }}</span>
        </div>
      </template>
      <div
        v-else
        class="text-xs text-muted"
      >
        No avatar detected yet
      </div>
    </ConnectionStatus>
    <OpenShockStatus :reference="reference" />
    <IntifaceStatus :reference="reference" />
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
