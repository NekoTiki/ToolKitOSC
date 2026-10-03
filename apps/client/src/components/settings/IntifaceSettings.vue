<script setup lang="ts">
import type { StatusInfo } from '@renderer/components/settings/StatusLine.vue'
import StatusLine from '@renderer/components/settings/StatusLine.vue'
import FormSection from '@renderer/components/ui/FormSection.vue'
import { useIntiface } from '@renderer/composables/useIntiface'
import { computed, ref, watch } from 'vue'

const {
  url,
  defaultUrl,
  isCustomUrl,
  setUrl,
  enabled,
  setEnabled,
  status,
  devices,
  installed,
  launch,
  autoLaunch,
  setAutoLaunch,
  launchAttempts,
  maxLaunchAttempts,
  refusedReason
} = useIntiface()

// Auto-launch has used all its tries and Intiface still isn't there.
const launchesUsedUp = computed(() => autoLaunch.value && status.value === 'error' && launchAttempts.value >= maxLaunchAttempts)

const draft = ref(url.value)

watch(url, (value) => (draft.value = value))

const commit = (): void => setUrl(draft.value)

const reset = (): void => {
  setUrl(undefined)
  draft.value = ''
}

const launching = ref(false)

const launchCentral = async (): Promise<void> => {
  launching.value = true

  try {
    await launch()
  } finally {
    launching.value = false
  }
}

const statusInfo = computed<StatusInfo>(() => {
  switch (status.value) {
    case 'connecting':
      return { icon: 'i-lucide-circle-dashed', class: 'text-muted animate-pulse', label: 'Connecting…' }
    case 'launching':
      return {
        icon: 'i-lucide-rocket',
        class: 'text-muted animate-pulse',
        label: `Launching Intiface Central (try ${launchAttempts.value} of ${maxLaunchAttempts})…`
      }
    case 'connected':
      return { icon: 'i-lucide-circle-check', class: 'text-success', label: 'Connected' }
    case 'error':
      return { icon: 'i-lucide-circle-x', class: 'text-error', label: 'Connection failed' }
    case 'refused':
      return { icon: 'i-lucide-ban', class: 'text-error', label: 'Intiface refused the connection' }
    case 'busy':
      return { icon: 'i-lucide-ban', class: 'text-error', label: 'Intiface Central isn\'t accepting connections' }
    default:
      return { icon: 'i-lucide-circle-dashed', class: 'text-muted', label: 'Disabled' }
  }
})

const deviceList = computed(() => Array.from(devices.value.values()))

const batteryTone = (level: number): string => (level <= 15 ? 'bg-error/15 text-error' : level <= 40 ? 'bg-warning/15 text-warning' : 'bg-success/15 text-success')
</script>

<template>
  <FormSection>
    <USwitch
      :model-value="enabled"
      label="Enable Intiface"
      description="Connects to Intiface Central to control toys."
      @update:model-value="setEnabled(!!$event)"
    />

    <template v-if="enabled">
      <UFormField
        label="Server URL"
        :description="isCustomUrl ? undefined : `Using the default (${defaultUrl})`"
      >
        <div class="flex gap-2">
          <UInput
            v-model="draft"
            :placeholder="defaultUrl"
            class="w-full"
            autocomplete="off"
            :ui="{ base: 'font-mono' }"
            @change="commit"
            @keyup.enter="commit"
            @blur="commit"
          />
          <UButton
            v-if="isCustomUrl"
            icon="i-lucide-rotate-ccw"
            color="neutral"
            variant="subtle"
            @click="reset"
          >
            Reset
          </UButton>
        </div>
      </UFormField>
      <USwitch
        :model-value="autoLaunch"
        :disabled="!installed"
        label="Launch Intiface Central automatically"
        :description="
          installed
            ? `Opens Intiface Central when it isn't running, up to ${maxLaunchAttempts} tries.`
            : 'Intiface Central wasn\'t found on this computer.'
        "
        @update:model-value="setAutoLaunch(!!$event)"
      />
      <StatusLine :status="statusInfo" />

      <UAlert
        v-if="status === 'refused'"
        color="error"
        variant="subtle"
        icon="i-lucide-ban"
        title="Intiface Central is running but refused the connection"
        :description="refusedReason || 'Another app may already be connected to it. Disconnect it in Intiface Central.'"
      />
      <UAlert
        v-else-if="status === 'busy'"
        color="error"
        variant="subtle"
        icon="i-lucide-ban"
        title="Intiface Central is open but isn't accepting connections"
        description="Another app may already be connected to it, or its server is stopped. Disconnect the other app or start the server in Intiface Central."
      />
      <UAlert
        v-else-if="launchesUsedUp"
        color="warning"
        variant="subtle"
        icon="i-lucide-triangle-alert"
        :title="`Couldn't start Intiface Central after ${maxLaunchAttempts} tries`"
        description="Make sure its server starts on its own in Intiface Central's settings, or start it there."
        :actions="installed ? [{ label: 'Launch Intiface Central', loading: launching, onClick: () => void launchCentral() }] : []"
      />
      <UAlert
        v-else-if="installed && !autoLaunch && (status === 'connecting' || status === 'error')"
        color="warning"
        variant="subtle"
        icon="i-lucide-triangle-alert"
        title="Intiface Central doesn't seem to be running"
        :actions="[{ label: 'Launch Intiface Central', loading: launching, onClick: () => void launchCentral() }]"
      />
    </template>
  </FormSection>

  <FormSection
    v-if="enabled && status === 'connected'"
    title="Toys"
    :description="`${deviceList.length} connected`"
  >
    <div class="grid divide-y divide-(--ui-border)">
      <div
        v-for="device in deviceList"
        :key="device.index"
        class="flex min-h-13 items-center gap-3"
      >
        <span class="grid size-10 place-items-center rounded-md bg-(--aurora-well) text-muted">
          <UIcon
            name="mdi:vibrate"
            class="size-5"
          />
        </span>
        <div class="grid min-w-0 flex-1">
          <b class="truncate font-medium text-highlighted">{{ device.name }}</b>
          <small class="truncate text-xs text-muted">{{ device.actuators.map((a) => a.description).join(', ') }}</small>
        </div>
        <span
          v-if="device.battery !== null"
          class="rounded-full px-2.5 py-0.5 text-xs font-medium tabular-nums"
          :class="batteryTone(device.battery)"
        >{{ device.battery }}%</span>
      </div>
      <p
        v-if="!deviceList.length"
        class="py-1 text-sm text-muted"
      >
        No toys connected. Pair one in Intiface Central.
      </p>
    </div>
  </FormSection>
</template>
