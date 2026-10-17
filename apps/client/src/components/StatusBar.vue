<script setup lang="ts">
import AddonsStatusChip from '@renderer/components/AddonsStatusChip.vue'
import type { StatusTone } from '@renderer/components/layout/StatusChip.vue'
import { useOscConnection } from '@renderer/composables/useOscConnection'
import { uniqueClientCount, unsupportedControlTypes, useWebsocketHost } from '@renderer/composables/useWebsocketHost'
import { api } from '@renderer/lib/tauri-bridge'
import { computed, onBeforeUnmount, onMounted } from 'vue'
import { useRouter } from 'vue-router'

// The top bar's connection chips. Each one opens the page where that connection is configured and
// its details live (no popovers - one tap, VR-friendly), as in the redesign. The exception is
// Add-ons, which groups every add-on that's on into one chip with a popover (opened by a tap).
const { status, open, close } = useWebsocketHost()
const { connected: oscConnected } = useOscConnection()
const router = useRouter()

const serverTone = computed<StatusTone>(() =>
  status.value === 'OPEN' ? 'success' : status.value === 'CONNECTING' ? 'warning' : 'error'
)

const go = (to: string): void => void router.push(to)

// A shock Rust couldn't deliver (see src-tauri/src/openshock). StatusBar is always mounted, so the
// listener is registered once.
const toast = useToast()

api.onOpenShockError((message) => toast.add({ title: 'OpenShock', description: message, color: 'error' }))

onMounted(open)
onBeforeUnmount(close)
</script>

<template>
  <div class="flex items-center gap-2.5">
    <StatusChip
      icon="i-lucide-globe"
      label="Server"
      status-dot
      :tone="serverTone"
      :warning="unsupportedControlTypes.length > 0"
      @click="go('/settings/account')"
    />
    <!-- The OSC link to VRChat, named after the game so it's clear what it's about. -->
    <StatusChip
      icon="i-lucide-gamepad-2"
      label="VRChat"
      status-dot
      :title="oscConnected ? 'Connected to VRChat over OSC' : 'Not connected to VRChat. Is the game running with OSC on?'"
      :tone="oscConnected ? 'success' : 'error'"
      @click="go('/parameters')"
    />
    <!-- Only while at least one add-on is on. -->
    <AddonsStatusChip />
    <StatusChip
      icon="i-lucide-users"
      :tone="uniqueClientCount ? 'success' : 'muted'"
      @click="go('/viewers')"
    >
      <span class="font-mono">{{ uniqueClientCount }}</span>
    </StatusChip>
  </div>
</template>
