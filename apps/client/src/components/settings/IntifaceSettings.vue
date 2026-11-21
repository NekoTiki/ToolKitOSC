<script setup lang="ts">
import IntifaceServerForm from '@renderer/components/settings/IntifaceServerForm.vue'
import type { StatusInfo } from '@renderer/components/settings/StatusLine.vue'
import StatusLine from '@renderer/components/settings/StatusLine.vue'
import FormSection from '@renderer/components/ui/FormSection.vue'
import { useAreYouSureModal } from '@renderer/composables/useAreYouSureModal'
import type { IntifaceServer } from '@renderer/composables/useIntiface'
import { serverName, useIntiface } from '@renderer/composables/useIntiface'
import { computed, ref } from 'vue'

const {
  enabled,
  servers,
  defaultPort,
  addServer,
  updateServer,
  removeServer,
  devices,
  showServers,
  toyServerName,
  installed,
  launch,
  autoLaunch,
  setAutoLaunch,
  maxLaunchAttempts
} = useIntiface()
const { openModal: confirm } = useAreYouSureModal()

// Which form is open: 'add', a server's id while editing it, or none.
const editing = ref<string | null>(null)

const hasLocal = computed(() => servers.value.some((server) => server.local))

const remove = async (server: IntifaceServer): Promise<void> => {
  const ok = await confirm({
    title: `Remove ${serverName(server)}?`,
    message: 'Its toys stop, and controls and automations on them show as unavailable until you add it again.',
    confirmText: 'Remove'
  })

  if (ok) await removeServer(server.id)
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

const toyCount = (server: IntifaceServer): string =>
  server.devices.length === 1 ? '1 toy' : server.devices.length ? `${server.devices.length} toys` : 'no toys yet'

const statusInfo = (server: IntifaceServer): StatusInfo => {
  switch (server.status) {
    case 'connecting':
      return { icon: 'i-lucide-circle-dashed', class: 'text-muted animate-pulse', label: 'Connecting…' }
    case 'launching':
      return {
        icon: 'i-lucide-rocket',
        class: 'text-muted animate-pulse',
        label: `Launching Intiface Central (try ${server.launchAttempts} of ${maxLaunchAttempts})…`
      }
    case 'connected':
      return { icon: 'i-lucide-circle-check', class: 'text-success', label: `Connected · ${toyCount(server)}` }
    case 'error':
      return { icon: 'i-lucide-circle-x', class: 'text-error', label: server.local ? 'Connection failed' : 'Not reachable. Is Intiface running there, on the same network?' }
    case 'refused':
      return { icon: 'i-lucide-ban', class: 'text-error', label: 'Intiface refused the connection' }
    case 'busy':
      return { icon: 'i-lucide-ban', class: 'text-error', label: 'Intiface Central isn\'t accepting connections' }
    default:
      return { icon: 'i-lucide-circle-dashed', class: 'text-muted', label: 'Disabled' }
  }
}

// Auto-launch has used all its tries and Intiface still isn't there.
const launchesUsedUp = (server: IntifaceServer): boolean =>
  server.local && autoLaunch.value && server.status === 'error' && server.launchAttempts >= maxLaunchAttempts

const deviceList = computed(() => Array.from(devices.value.values()))

const batteryTone = (level: number): string => (level <= 15 ? 'bg-error/15 text-error' : level <= 40 ? 'bg-warning/15 text-warning' : 'bg-success/15 text-success')
</script>

<template>
  <!-- Turned on and off on the Add-ons page: this section only shows while Intiface toys is on. -->
  <template v-if="enabled">
    <FormSection
      title="Servers"
      description="Every Intiface you connect to at once: this PC, a phone running Intiface Central…"
    >
      <template #actions>
        <UButton
          icon="i-lucide-plus"
          color="neutral"
          variant="subtle"
          :disabled="editing === 'add'"
          @click="editing = 'add'"
        >
          Add a server
        </UButton>
      </template>

      <IntifaceServerForm
        v-if="editing === 'add'"
        :default-port="defaultPort"
        submit-label="Add server"
        :save="addServer"
        @done="editing = null"
      />

      <p
        v-if="!servers.length && editing !== 'add'"
        class="text-sm text-muted"
      >
        No servers. Add one to use your toys.
      </p>

      <div
        v-else
        class="grid divide-y divide-(--ui-border)"
      >
        <div
          v-for="server in servers"
          :key="server.id"
          class="grid gap-2.5 py-3 first:pt-0 last:pb-0"
        >
          <IntifaceServerForm
            v-if="editing === server.id"
            :initial="server"
            :default-port="defaultPort"
            submit-label="Save"
            :save="(label, host, port) => updateServer(server.id, label, host, port)"
            @done="editing = null"
          />
          <div
            v-else
            class="flex items-center gap-3"
          >
            <span class="grid size-10 shrink-0 place-items-center rounded-md bg-(--aurora-well) text-muted">
              <UIcon
                :name="server.local ? 'i-lucide-monitor' : 'i-lucide-smartphone'"
                class="size-5"
              />
            </span>
            <div class="grid min-w-0 flex-1 gap-0.5">
              <b class="truncate font-medium text-highlighted">{{ serverName(server) }}</b>
              <small class="truncate font-mono text-xs text-muted">{{ server.host }}:{{ server.port }}</small>
              <StatusLine :status="statusInfo(server)" />
            </div>
            <UButton
              icon="i-lucide-pencil"
              color="neutral"
              variant="subtle"
              :aria-label="`Edit ${serverName(server)}`"
              @click="editing = server.id"
            >
              Edit
            </UButton>
            <UButton
              icon="i-lucide-trash-2"
              color="neutral"
              variant="subtle"
              :aria-label="`Remove ${serverName(server)}`"
              @click="remove(server)"
            >
              Remove
            </UButton>
          </div>

          <UAlert
            v-if="server.status === 'refused'"
            color="error"
            variant="subtle"
            icon="i-lucide-ban"
            title="Intiface Central is running but refused the connection"
            :description="server.refusedReason || 'Another app may already be connected to it. Disconnect it in Intiface Central.'"
          />
          <UAlert
            v-else-if="server.status === 'busy'"
            color="error"
            variant="subtle"
            icon="i-lucide-ban"
            title="Intiface Central is open but isn't accepting connections"
            description="Another app may already be connected to it, or its server is stopped. Disconnect the other app or start the server in Intiface Central."
          />
          <UAlert
            v-else-if="launchesUsedUp(server)"
            color="warning"
            variant="subtle"
            icon="i-lucide-triangle-alert"
            :title="`Couldn't start Intiface Central after ${maxLaunchAttempts} tries`"
            description="Make sure its server starts on its own in Intiface Central's settings, or start it there."
            :actions="installed ? [{ label: 'Launch Intiface Central', loading: launching, onClick: () => void launchCentral() }] : []"
          />
          <UAlert
            v-else-if="server.local && installed && !autoLaunch && (server.status === 'connecting' || server.status === 'error')"
            color="warning"
            variant="subtle"
            icon="i-lucide-triangle-alert"
            title="Intiface Central doesn't seem to be running"
            :actions="[{ label: 'Launch Intiface Central', loading: launching, onClick: () => void launchCentral() }]"
          />
        </div>
      </div>

      <USwitch
        v-if="hasLocal"
        :model-value="autoLaunch"
        :disabled="!installed"
        label="Launch Intiface Central automatically"
        :description="
          installed
            ? `Opens Intiface Central on this PC when it isn't running, up to ${maxLaunchAttempts} tries.`
            : 'Intiface Central wasn\'t found on this computer.'
        "
        @update:model-value="setAutoLaunch(!!$event)"
      />
    </FormSection>

    <FormSection
      v-if="servers.some((server) => server.status === 'connected')"
      title="Toys"
      :description="`${deviceList.length} connected`"
    >
      <div class="grid divide-y divide-(--ui-border)">
        <div
          v-for="device in deviceList"
          :key="`${device.server}:${device.index}`"
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
            <small class="truncate text-xs text-muted">
              <template v-if="showServers">{{ toyServerName(device.server) }} · </template>{{ device.actuators.map((a) => a.description).join(', ') }}
            </small>
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
</template>
