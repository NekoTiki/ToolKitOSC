<script setup lang="ts">
import type { StatusInfo } from '@renderer/components/settings/StatusLine.vue'
import StatusLine from '@renderer/components/settings/StatusLine.vue'
import FormSection from '@renderer/components/ui/FormSection.vue'
import { useSteamVrLaunch } from '@renderer/composables/useSteamVrLaunch'
import { useTraySettings } from '@renderer/composables/useTraySettings'
import { useVrcxLaunch } from '@renderer/composables/useVrcxLaunch'
import { computed } from 'vue'

const { enabled: trayEnabled, setEnabled: setTrayEnabled } = useTraySettings()
const { enabled: steamVrEnabled, setEnabled: setSteamVrEnabled, status: steamVrStatus, error: steamVrError, available: steamVrAvailable } = useSteamVrLaunch()
const { enabled: vrcxEnabled, setEnabled: setVrcxEnabled, status: vrcxStatus, error: vrcxError, available: vrcxAvailable } = useVrcxLaunch()

const steamVrInfo = computed<StatusInfo>(() => {
  switch (steamVrStatus.value) {
    case 'checking':
      return { icon: 'i-lucide-circle-dashed', class: 'text-muted animate-pulse', label: 'Registering with SteamVR…' }
    case 'enabled':
      return { icon: 'i-lucide-circle-check', class: 'text-success', label: 'Registered' }
    case 'error':
      return { icon: 'i-lucide-circle-x', class: 'text-error', label: steamVrError.value || 'Failed' }
    default:
      return { icon: 'i-lucide-circle-dashed', class: 'text-muted', label: 'Disabled' }
  }
})

const vrcxInfo = computed<StatusInfo>(() => {
  switch (vrcxStatus.value) {
    case 'enabled':
      return { icon: 'i-lucide-circle-check', class: 'text-success', label: 'Enabled' }
    case 'error':
      return { icon: 'i-lucide-circle-x', class: 'text-error', label: vrcxError.value || 'Failed' }
    default:
      return { icon: 'i-lucide-circle-dashed', class: 'text-muted', label: 'Disabled' }
  }
})
</script>

<template>
  <FormSection>
    <USwitch
      :model-value="trayEnabled"
      label="Keep running in the tray"
      description="Closing the window keeps the app running in the system tray instead of quitting."
      @update:model-value="setTrayEnabled(!!$event)"
    />
  </FormSection>

  <FormSection v-if="steamVrAvailable">
    <USwitch
      :model-value="steamVrEnabled"
      label="Launch with SteamVR"
      description="Starts this app automatically whenever SteamVR starts."
      @update:model-value="setSteamVrEnabled(!!$event)"
    />
    <StatusLine
      v-if="steamVrEnabled"
      :status="steamVrInfo"
    />
  </FormSection>

  <FormSection v-if="vrcxAvailable">
    <USwitch
      :model-value="vrcxEnabled"
      label="Launch with VRChat"
      description="Adds this app to VRCX's auto-launch folder, so it starts whenever VRChat does."
      @update:model-value="setVrcxEnabled(!!$event)"
    />
    <StatusLine
      v-if="vrcxEnabled"
      :status="vrcxInfo"
    />
  </FormSection>
</template>
