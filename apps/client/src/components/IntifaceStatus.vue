<script setup lang="ts">
import type { IntifaceStatus } from '@renderer/composables/useIntiface'
import { useIntiface } from '@renderer/composables/useIntiface'
import { computed, ref } from 'vue'

const { status, devices, installed, launch } = useIntiface()

const launching = ref(false)

const launchIntiface = async (): Promise<void> => {
  launching.value = true

  try {
    await launch()
  } finally {
    launching.value = false
  }
}

const STATE_COLOR: Record<IntifaceStatus, string> = {
  connected: 'text-success',
  connecting: 'text-warning',
  error: 'text-error',
  disabled: 'text-muted'
}

const STATE_TEXT: Record<IntifaceStatus, string> = {
  connected: 'Connected',
  connecting: 'Connecting…',
  error: 'Connection failed',
  disabled: 'Disabled'
}

const deviceList = computed(() => Array.from(devices.value.values()))

const batteryIcon = (level: number): string => {
  if (level <= 15) return 'i-lucide-battery-warning'
  if (level <= 40) return 'i-lucide-battery-low'
  if (level <= 80) return 'i-lucide-battery-medium'

  return 'i-lucide-battery-full'
}
</script>

<template>
  <!-- A popover, not a plain tooltip like ConnectionStatus - there's a variable-length device (and
  battery) list to show once connected, which doesn't fit a one-line hover label.
  z-20: same reasoning as ConnectionStatus.vue - this renders inside AppHeader.vue's sticky z-10
  bar and needs its own explicit z-index to float above it. -->
  <UPopover
    mode="hover"
    :content="{ align: 'center', side: 'bottom' }"
    :ui="{ content: 'z-20' }"
  >
    <UIcon
      name="mdi:vibrate"
      class="size-5"
      :class="STATE_COLOR[status]"
    />

    <template #content>
      <div class="flex min-w-52 flex-col gap-2 p-3">
        <div
          class="flex items-center gap-1.5 text-sm font-medium"
          :class="STATE_COLOR[status]"
        >
          <UIcon
            name="mdi:vibrate"
            class="size-4"
          />
          Intiface: {{ STATE_TEXT[status] }}
        </div>

        <template v-if="status === 'connected'">
          <div
            v-if="deviceList.length === 0"
            class="text-xs text-muted"
          >
            No toys connected
          </div>
          <div
            v-else
            class="flex flex-col gap-1.5"
          >
            <div
              v-for="device in deviceList"
              :key="device.index"
              class="flex items-center justify-between gap-3 text-sm"
            >
              <span class="truncate">{{ device.name }}</span>
              <span
                v-if="device.battery !== null"
                class="flex shrink-0 items-center gap-1 text-xs text-muted"
              >
                <UIcon
                  :name="batteryIcon(device.battery)"
                  class="size-4"
                />
                {{ device.battery }}%
              </span>
            </div>
          </div>
        </template>

        <div
          v-else-if="(status === 'connecting' || status === 'error') && installed"
          class="flex flex-col gap-2"
        >
          <p class="text-xs text-muted">
            Intiface Central doesn't appear to be running.
          </p>
          <UButton
            size="xs"
            icon="i-lucide-play"
            :loading="launching"
            @click="launchIntiface"
          >
            Launch Intiface Central
          </UButton>
        </div>
      </div>
    </template>
  </UPopover>
</template>

<style scoped></style>
