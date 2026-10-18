<script setup lang="ts">
import BhapticsMotors from '@renderer/components/bhaptics/BhapticsMotors.vue'
import type { StatusInfo } from '@renderer/components/settings/StatusLine.vue'
import StatusLine from '@renderer/components/settings/StatusLine.vue'
import FormSection from '@renderer/components/ui/FormSection.vue'
import { BHAPTICS_POSITIONS, useBhaptics } from '@renderer/composables/useBhaptics'
import type { BhapticsDeviceCheck, BhapticsPosition } from '@renderer/lib/tauri-bridge'
import { api } from '@renderer/lib/tauri-bridge'
import { computed, ref } from 'vue'

// Settings > bHaptics: the link to bHaptics Player, who reads avatar touch, a check of the worn
// avatar and each device live, with its strength and Test motors.
const { settings, state, change, strength } = useBhaptics()
const toast = useToast()
// The simulated Player only exists in development builds: its Rust module isn't compiled into a
// release, the command refuses there, and this card isn't shown (see bhaptics/simulator.rs).
const dev = import.meta.env.DEV

const simulate = (on: boolean): void => {
  api.bhapticsSimulate(on).catch((error) => toast.add({ title: 'Could not switch the simulated Player', description: String(error), color: 'error' }))
}

const statusInfo = computed<StatusInfo>(() => {
  switch (state.value.status) {
    case 'connecting':
      return { icon: 'i-lucide-circle-dashed', class: 'text-muted animate-pulse', label: 'Connecting…' }
    case 'connected':
      return {
        icon: 'i-lucide-circle-check',
        class: 'text-success',
        label: state.value.simulated ? 'Connected to the simulated Player' : 'bHaptics Player connected'
      }
    case 'not-running':
      return { icon: 'i-lucide-circle-x', class: 'text-error', label: 'bHaptics Player isn\'t running' }
    default:
      return { icon: 'i-lucide-circle-dashed', class: 'text-muted', label: 'Off' }
  }
})

const launching = ref(false)

const launchPlayer = async (): Promise<void> => {
  launching.value = true

  try {
    await api.bhapticsLaunchPlayer()
  } catch (error) {
    toast.add({ title: 'Could not start bHaptics Player', description: String(error), color: 'error' })
  } finally {
    launching.value = false
  }
}

const ours = computed(() => settings.value?.touchSource === 'toolkitosc')
// Player plays something we didn't send while we read touch: most likely its own VRChat handling.
const doubled = computed(() => ours.value && state.value.otherKeys.length > 0)

const avatar = computed(() => state.value.avatar)
const SCHEMES = { v2: 'bOSC/v2', v2m: 'bOSC/v2m (Quest)', v1: 'bOSC v1 (older avatars)', legacy: 'bHapticsOSC (older avatars)' }

interface CheckRow {
  key: string
  label: string
  tone: 'ok' | 'warn' | 'off'
  detail: string
}

const deviceRow = (device: BhapticsDeviceCheck): CheckRow => {
  const label = BHAPTICS_POSITIONS[device.position].label
  const missing = device.motors - device.receivers
  const connected = state.value.connected.includes(device.position)

  if (missing > 0) {
    return { key: device.position, label, tone: 'warn', detail: `${device.receivers} / ${device.motors} motors. ${missing} have no receiver on this avatar.` }
  }

  if (!connected) return { key: device.position, label, tone: 'warn', detail: `${device.motors} / ${device.motors} motors, but not connected in bHaptics Player` }

  return { key: device.position, label, tone: 'ok', detail: `${device.motors} / ${device.motors} motors` }
}

const checks = computed<CheckRow[]>(() => {
  const check = avatar.value

  if (!check) return []

  const rows: CheckRow[] = check.devices.map(deviceRow)
  const missing = (Object.keys(BHAPTICS_POSITIONS) as BhapticsPosition[]).filter(
    (position) => state.value.connected.includes(position) && !check.devices.some((device) => device.position === position)
  )

  if (missing.length) {
    rows.push({
      key: 'not-on-avatar',
      label: missing.map((position) => BHAPTICS_POSITIONS[position].label).join(', '),
      tone: 'off',
      detail: 'Connected, but this avatar has no receivers for them'
    })
  }

  if (check.spsZones) {
    rows.push({
      key: 'sps',
      label: 'SPS',
      tone: 'off',
      detail: `${check.spsZones} touch zone${check.spsZones === 1 ? '' : 's'} on this avatar`
    })
  }

  return rows
})

const TONE: Record<CheckRow['tone'], { name: string; class: string }> = {
  ok: { name: 'i-lucide-circle-check', class: 'text-success' },
  warn: { name: 'i-lucide-triangle-alert', class: 'text-warning' },
  off: { name: 'i-lucide-circle-dashed', class: 'text-muted' }
}

// Every position that's connected, on the avatar, or playing, in body order.
const positions = computed(() =>
  (Object.keys(BHAPTICS_POSITIONS) as BhapticsPosition[]).filter(
    (position) =>
      state.value.connected.includes(position) || state.value.playing[position] || avatar.value?.devices.some((device) => device.position === position)
  )
)

const setStrength = (position: BhapticsPosition, value: number): void =>
  change((target) => {
    target.positions[position] = { strength: value / 100 }
  })

const testMotors = async (position: BhapticsPosition): Promise<void> => {
  try {
    await api.bhapticsTestMotors(position)
  } catch (error) {
    toast.add({ title: 'Could not test the motors', description: String(error), color: 'error' })
  }
}

const copyDiagnostics = async (): Promise<void> => {
  try {
    await navigator.clipboard.writeText(await api.bhapticsDiagnostics())
    toast.add({ title: 'Diagnostics copied', icon: 'i-lucide-check', color: 'success' })
  } catch {
    toast.add({ title: 'Could not copy the diagnostics', color: 'error' })
  }
}
</script>

<template>
  <UAlert
    color="info"
    variant="subtle"
    icon="i-lucide-flask-conical"
    title="bHaptics is in beta"
    description="It hasn't been tried on a real vest yet, so the motor order and the effects may still change."
  />

  <section
    v-if="dev && settings"
    class="grid gap-3 rounded-panel border-2 border-dashed border-warning/60 bg-warning/8 p-4.5"
  >
    <div class="flex flex-wrap items-center gap-2.5">
      <UIcon
        name="i-lucide-test-tube-diagonal"
        class="size-5 text-warning"
      />
      <h2 class="flex-1 text-[17px] font-semibold text-highlighted">
        Simulated bHaptics Player
      </h2>
      <UBadge
        label="Dev build only"
        color="warning"
        variant="solid"
        class="rounded-full"
      />
    </div>
    <p class="text-[13px] leading-snug text-muted">
      A pretend Player with a vest, a Tactal and both arms connected, so you can test without gear. Only in development builds:
      released versions don't include it.
    </p>
    <USwitch
      :model-value="state.simulated"
      label="Use the simulated Player"
      @update:model-value="simulate(!!$event)"
    />
  </section>

  <FormSection v-if="settings">
    <div class="flex flex-wrap items-center gap-2">
      <StatusLine :status="statusInfo" />
      <UBadge
        v-if="state.simulated"
        label="Simulated"
        color="warning"
        variant="subtle"
        size="sm"
        class="rounded-full"
      />
    </div>
    <UAlert
      v-if="state.status === 'not-running'"
      color="warning"
      variant="subtle"
      icon="i-lucide-triangle-alert"
      title="bHaptics Player isn't running"
      :description="state.installed ? undefined : 'Open bHaptics Player and pair your gear in it. The app connects on its own once it runs.'"
      :actions="[
        ...(state.installed ? [{ label: 'Start bHaptics Player', loading: launching, onClick: () => void launchPlayer() }] : []),
        ...(dev ? [{ label: 'Use the simulated Player', color: 'warning' as const, variant: 'outline' as const, onClick: () => simulate(true) }] : [])
      ]"
    />
    <UAlert
      v-if="doubled"
      color="warning"
      variant="subtle"
      icon="i-lucide-copy"
      title="Your gear may feel every touch twice"
      description="bHaptics Player is playing touch on its own too. Turn off its VRChat OSC handling in bHaptics Player, or let it handle touch below."
    />

    <USwitch
      :model-value="settings.enabled"
      label="Avatar touch plays on your gear"
      description="Also TKOSC/bHaptics in your expression menu. Viewers and automations still play when it's off."
      @update:model-value="change((target) => (target.enabled = !!$event))"
    />
    <USwitch
      :model-value="ours"
      label="Read avatar touch in ToolKitOSC"
      description="Turn off bHaptics Player's own VRChat OSC handling first. Off: bHaptics Player handles touch, and the app only adds viewers and automations."
      @update:model-value="change((target) => (target.touchSource = $event ? 'toolkitosc' : 'player'))"
    />
    <USwitch
      :model-value="settings.touchFrom.self"
      :disabled="!ours"
      label="Feel your own hands"
      description="Off: only other people's touches play."
      @update:model-value="change((target) => (target.touchFrom.self = !!$event))"
    />
  </FormSection>

  <FormSection
    v-if="avatar"
    :title="`Current avatar · ${avatar.name}`"
    :description="
      avatar.schemes.length
        ? `bHaptics package found: ${avatar.schemes.map((scheme) => SCHEMES[scheme]).join(', ')}`
        : 'This avatar has no bHaptics receivers. Add them with bHaptics\' VRChat package.'
    "
  >
    <ul
      v-if="checks.length"
      class="grid gap-2"
    >
      <li
        v-for="row in checks"
        :key="row.key"
        class="grid grid-cols-[1.25rem_minmax(0,1fr)] gap-2 text-[13px]"
      >
        <UIcon
          :name="TONE[row.tone].name"
          class="mt-0.5 size-4"
          :class="TONE[row.tone].class"
        />
        <span class="text-muted"><b class="font-semibold text-highlighted">{{ row.label }}</b> · {{ row.detail }}</span>
      </li>
    </ul>
  </FormSection>

  <FormSection
    title="Devices"
    description="What each one plays right now. Strength is on top of the intensity you set in bHaptics Player."
  >
    <template #actions>
      <UButton
        icon="i-lucide-clipboard-copy"
        color="neutral"
        variant="subtle"
        @click="copyDiagnostics"
      >
        Copy diagnostics
      </UButton>
    </template>
    <p
      v-if="!positions.length"
      class="text-sm text-muted"
    >
      Nothing connected yet. Pair your gear in bHaptics Player.
    </p>
    <div class="grid gap-3">
      <div
        v-for="position in positions"
        :key="position"
        class="grid grid-cols-[auto_minmax(0,1fr)] items-center gap-4 max-sm:grid-cols-1"
      >
        <BhapticsMotors
          :position="position"
          :levels="state.playing[position]"
        />
        <div class="grid min-w-0 gap-2">
          <div class="flex flex-wrap items-center gap-2 text-sm">
            <b class="font-semibold text-highlighted">{{ BHAPTICS_POSITIONS[position].label }}</b>
            <UBadge
              v-if="state.touched.includes(position)"
              color="warning"
              variant="subtle"
              size="sm"
            >
              Being touched
            </UBadge>
            <UBadge
              v-else-if="!state.connected.includes(position)"
              color="neutral"
              variant="subtle"
              size="sm"
            >
              Not connected
            </UBadge>
            <span class="ml-auto font-semibold text-highlighted tabular-nums">{{ Math.round(strength(position) * 100) }}%</span>
          </div>
          <USlider
            :model-value="Math.round(strength(position) * 100)"
            :min="0"
            :max="100"
            :step="5"
            size="lg"
            :aria-label="`${BHAPTICS_POSITIONS[position].label} strength`"
            @update:model-value="setStrength(position, $event ?? 100)"
          />
          <UButton
            class="justify-self-start"
            icon="i-lucide-scan-line"
            color="neutral"
            variant="subtle"
            size="sm"
            :loading="state.testing === position"
            :disabled="state.status !== 'connected'"
            @click="testMotors(position)"
          >
            Test motors
          </UButton>
        </div>
      </div>
    </div>
  </FormSection>

  <FormSection
    title="Avatar parameters"
    description="Add this to your avatar to use it in VRChat."
  >
    <dl class="grid gap-2 text-sm">
      <div class="grid gap-0.5 rounded-field bg-(--aurora-well) px-3.5 py-2.5">
        <dt><code class="font-semibold text-highlighted">TKOSC/bHaptics</code> <span class="text-muted">· bool</span></dt>
        <dd class="text-muted">
          Turns avatar touch on and off from your expression menu, like <b class="text-highlighted">Avatar touch plays on your gear</b> here.
        </dd>
      </div>
    </dl>
  </FormSection>
</template>
