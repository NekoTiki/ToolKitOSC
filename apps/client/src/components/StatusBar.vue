<script setup lang="ts">
import type { StatusTone } from '@renderer/components/layout/StatusChip.vue'
import type { IntifaceStatus } from '@renderer/composables/useIntiface'
import { useIntiface } from '@renderer/composables/useIntiface'
import type { OpenShockStatus } from '@renderer/composables/useOpenShock'
import { useOpenShock } from '@renderer/composables/useOpenShock'
import { useOscConnection } from '@renderer/composables/useOscConnection'
import { uniqueClientCount, unsupportedControlTypes, useWebsocketHost } from '@renderer/composables/useWebsocketHost'
import { computed, onBeforeUnmount, onMounted } from 'vue'
import { useRouter } from 'vue-router'

// The top bar's connection chips. Each one opens the page where that connection is configured and
// its details live (no popovers - one tap, VR-friendly), as in the redesign.
const { status, open, close } = useWebsocketHost()
const { connected: oscConnected } = useOscConnection()
const { status: openShockStatus } = useOpenShock()
const { status: intifaceStatus } = useIntiface()
const router = useRouter()

const serverTone = computed<StatusTone>(() =>
  status.value === 'OPEN' ? 'success' : status.value === 'CONNECTING' ? 'warning' : 'error'
)

const OPENSHOCK_TONE: Record<OpenShockStatus, StatusTone> = {
  valid: 'success',
  checking: 'warning',
  invalid: 'error',
  disabled: 'muted',
  unconfigured: 'muted'
}

const INTIFACE_TONE: Record<IntifaceStatus, StatusTone> = {
  connected: 'success',
  connecting: 'warning',
  error: 'error',
  disabled: 'muted'
}

const go = (to: string): void => void router.push(to)

onMounted(open)
onBeforeUnmount(close)
</script>

<template>
  <div class="flex items-center gap-2.5">
    <StatusChip
      label="Server"
      :tone="serverTone"
      :warning="unsupportedControlTypes.length > 0"
      @click="go('/settings/account')"
    />
    <StatusChip
      label="OSC"
      :tone="oscConnected ? 'success' : 'error'"
      @click="go('/parameters')"
    />
    <StatusChip
      label="OpenShock"
      :tone="OPENSHOCK_TONE[openShockStatus]"
      @click="go('/settings/openshock')"
    />
    <StatusChip
      label="Intiface"
      :tone="INTIFACE_TONE[intifaceStatus]"
      @click="go('/settings/intiface')"
    />
    <StatusChip
      icon="i-lucide-users"
      :tone="uniqueClientCount ? 'success' : 'muted'"
      @click="go('/viewers')"
    >
      <span class="font-mono">{{ uniqueClientCount }}</span>
    </StatusChip>
  </div>
</template>
