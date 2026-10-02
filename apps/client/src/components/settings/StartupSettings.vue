<script setup lang="ts">
import type { StatusInfo } from '@renderer/components/settings/StatusLine.vue'
import StatusLine from '@renderer/components/settings/StatusLine.vue'
import FormSection from '@renderer/components/ui/FormSection.vue'
import { useSteamVrLaunch } from '@renderer/composables/useSteamVrLaunch'
import { useTraySettings } from '@renderer/composables/useTraySettings'
import { useVrcxLaunch } from '@renderer/composables/useVrcxLaunch'
import { computed } from 'vue'

const { enabled: trayEnabled, setEnabled: setTrayEnabled } = useTraySettings()
const { enabled: steamVrEnabled, setEnabled: setSteamVrEnabled, status: steamVrStatus, error: steamVrError, availability: steamVrAvailability, available: steamVrAvailable } = useSteamVrLaunch()
const { enabled: vrcxEnabled, setEnabled: setVrcxEnabled, status: vrcxStatus, error: vrcxError, availability: vrcxAvailability, available: vrcxAvailable } = useVrcxLaunch()

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

const steamVrDescription = computed(() =>
  steamVrAvailable.value
    ? 'Starts this app automatically whenever SteamVR starts.'
    : "SteamVR wasn't found on this computer. Install it from Steam, then restart this app."
)

const vrcxDescription = computed(() =>
  vrcxAvailable.value
    ? "Adds this app to VRCX's auto-launch folder, so it starts whenever VRChat does."
    : "Needs VRCX, which wasn't found on this computer. Install it, then restart this app."
)
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

  <!-- Hidden where the OS can't support the integration at all; shown but disabled (saying what's
       missing) where it could work but the app it hooks into isn't installed. -->
  <FormSection v-if="steamVrAvailability !== 'unsupported'">
    <USwitch
      :model-value="steamVrAvailable && steamVrEnabled"
      :disabled="!steamVrAvailable"
      label="Launch with SteamVR"
      :description="steamVrDescription"
      @update:model-value="setSteamVrEnabled(!!$event)"
    />
    <StatusLine
      v-if="steamVrAvailable && steamVrEnabled"
      :status="steamVrInfo"
    />
  </FormSection>

  <FormSection v-if="vrcxAvailability !== 'unsupported'">
    <USwitch
      :model-value="vrcxAvailable && vrcxEnabled"
      :disabled="!vrcxAvailable"
      label="Launch with VRChat"
      :description="vrcxDescription"
      @update:model-value="setVrcxEnabled(!!$event)"
    />
    <StatusLine
      v-if="vrcxAvailable && vrcxEnabled"
      :status="vrcxInfo"
    />
  </FormSection>
</template>
