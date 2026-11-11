<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from 'vue'

import { useOpenShockControl } from '../composables/useOpenShockControl'
import type { OpenShockControl } from '../types/controls'
import ControlBase from './ControlBase.vue'
import Fader from './Fader.vue'

// The tile for Fixed Shocker and Viewer Shocker controls (the rolled Shocker is ControlOpenShock.vue).
// Fixed shows what a press sends. Viewer Shocker lets whoever presses choose within the control's ranges:
// on a wide tile with two faders right on it, on a one-cell tile in a panel that opens from it
// (there's no room for faders a VR pointer could hit). Either way the shock goes out at once, and
// the button shows it running, then the cooldown draining.
const props = defineProps<{
  controlId: string
  title: string
  mode: OpenShockControl['mode']
  // A Viewer Shocker: whoever presses chooses.
  choice?: boolean
  intensity: OpenShockControl['intensity']
  duration: OpenShockControl['duration']
  wide?: boolean
}>()

const emit = defineEmits<{ (e: 'run', choice: { intensity?: number; duration?: number }): void }>()

const { lastUpdate } = useOpenShockControl()

const viewer = computed(() => !!props.choice)
const shock = computed(() => props.mode === 'Shock')
// Vibrations are blue instead of red: every error-colored part takes the info color instead.
const VIBRATE = '[--ui-error:var(--ui-info)]'

// The viewer's choice, kept for the next press. Starts at the gentlest the control allows.
const power = ref(props.intensity.min)
const time = ref(props.duration.min)

watch(
  () => [props.intensity, props.duration],
  () => {
    power.value = Math.min(props.intensity.max, Math.max(props.intensity.min, power.value))
    time.value = Math.min(props.duration.max, Math.max(props.duration.min, time.value))
  },
  { deep: true }
)

const seconds = (ms: number): string => (ms / 1000).toFixed(1)

// What the host reports it sent (the `shock-value` event, which the share page gets too): the shock
// runs for its duration, then the cooldown, which starts once it ends, drains.
const now = ref(Date.now())
const sendEnd = ref(0)
const sendTotal = ref(0)
const coolEnd = ref(0)
let ticker: ReturnType<typeof setInterval> | undefined

const stopTicker = (): void => {
  if (ticker) clearInterval(ticker)
  ticker = undefined
}

watch(
  () => lastUpdate.value,
  (update) => {
    if (!update || update.controlId !== props.controlId) return

    const start = Date.now()

    now.value = start
    sendTotal.value = Math.max(update.value.duration, 1)
    sendEnd.value = start + update.value.duration
    coolEnd.value = update.value.cooldownEnd

    stopTicker()
    ticker = setInterval(() => {
      now.value = Date.now()
      if (now.value >= sendEnd.value && now.value >= coolEnd.value) stopTicker()
    }, 100)
  },
  { deep: true }
)

onBeforeUnmount(stopTicker)

const phase = computed(() => (now.value < sendEnd.value ? 'sending' : now.value < coolEnd.value ? 'cooling' : 'idle'))

// The bar along the button's bottom edge: the shock running out, then the cooldown draining.
const bar = computed(() => {
  if (phase.value === 'sending') return (sendEnd.value - now.value) / sendTotal.value
  if (phase.value === 'cooling') return (coolEnd.value - now.value) / Math.max(coolEnd.value - sendEnd.value, 1)

  return 0
})

const busyLabel = computed(() =>
  phase.value === 'sending' ? (shock.value ? 'SHOCKING' : 'VIBRATING') : `${((coolEnd.value - now.value) / 1000).toFixed(1)}s`
)

const busyIcon = computed(() => (phase.value === 'sending' ? 'material-symbols:electric-bolt' : 'lucide:hourglass'))

// A one-cell viewer tile opens the panel instead of firing.
const choosing = ref(false)
const opensPanel = computed(() => viewer.value && !props.wide)

const fire = (): void => {
  if (phase.value !== 'idle') return

  emit('run', viewer.value ? { intensity: power.value, duration: time.value } : {})
  choosing.value = false
}

const onTileButton = (): void => {
  if (opensPanel.value) choosing.value = true
  else fire()
}

const readouts = computed(() =>
  viewer.value
    ? [
        { label: 'Up to', value: String(props.intensity.max), unit: '%' },
        { label: 'Up to', value: seconds(props.duration.max), unit: 's' }
      ]
    : [
        { label: 'Power', value: String(props.intensity.min), unit: '%' },
        { label: 'Time', value: seconds(props.duration.min), unit: 's' }
      ]
)
</script>

<template>
  <control-base
    :title="title"
    kind="danger"
    :class="{ 'shock-running': phase === 'sending', [VIBRATE]: !shock }"
  >
    <template #badge>
      <span
        class="grid size-5 shrink-0 place-items-center rounded-full bg-error/15 text-error"
        :title="mode"
      >
        <UIcon
          :name="shock ? 'material-symbols:electric-bolt' : 'lucide:waves'"
          class="size-3"
        />
      </span>
    </template>

    <template #middle>
      <!-- Wide viewer tile: the faders themselves, shrunk to fit two above the button. -->
      <div
        v-if="viewer && wide"
        class="grid w-full gap-1.5 [--fader-h:calc(var(--tile-control)*0.62)]"
      >
        <div class="flex items-center gap-2">
          <span class="grid w-14 shrink-0 gap-0.5 text-left">
            <span class="font-mono text-[10px] leading-none text-muted uppercase">Power</span>
            <span class="text-[calc(var(--tile-value)*0.55)] leading-none font-semibold text-highlighted tabular-nums">{{ power }}<span class="text-[0.6em] opacity-70">%</span></span>
          </span>
          <fader
            v-model="power"
            class="min-w-0 flex-1"
            label="Power"
            color="error"
            :min="intensity.min"
            :max="intensity.max"
            :value-text="`${power}%`"
          />
        </div>
        <div class="flex items-center gap-2">
          <span class="grid w-14 shrink-0 gap-0.5 text-left">
            <span class="font-mono text-[10px] leading-none text-muted uppercase">Time</span>
            <span class="text-[calc(var(--tile-value)*0.55)] leading-none font-semibold text-highlighted tabular-nums">{{ seconds(time) }}<span class="text-[0.6em] opacity-70">s</span></span>
          </span>
          <fader
            v-model="time"
            class="min-w-0 flex-1"
            label="Time"
            color="error"
            :min="duration.min"
            :max="duration.max"
            :step="100"
            :value-text="`${seconds(time)} seconds`"
          />
        </div>
      </div>

      <div
        v-else
        class="grid w-full grid-cols-2 gap-1.5"
      >
        <div
          v-for="(readout, i) in readouts"
          :key="i"
          class="grid min-w-0 gap-1 rounded-field well px-2 pt-[calc(var(--tile-pad)*0.45)] pb-[calc(var(--tile-pad)*0.6)]"
        >
          <span class="font-mono text-[10px] leading-none text-muted uppercase">{{ readout.label }}</span>
          <span class="truncate text-[calc(var(--tile-value)*0.78)] leading-none font-semibold text-highlighted tabular-nums">
            {{ readout.value }}<span class="text-[0.55em] opacity-70">{{ readout.unit }}</span>
          </span>
        </div>
      </div>
    </template>

    <!-- The whole tile isn't a trigger (unlike toggles): a shock takes a deliberate press on the
    button, not a stray pointer click anywhere on the tile. The panel can still be opened while
    cooling down, to choose the next one. -->
    <button
      type="button"
      class="relative flex h-(--tile-control) w-full cursor-pointer items-center justify-center gap-2 overflow-hidden rounded-field border border-error/55 bg-error/15 text-[calc(var(--tile-title)*0.95)] font-semibold tracking-wide text-error disabled:cursor-wait disabled:opacity-80"
      :disabled="!opensPanel && phase !== 'idle'"
      @click.stop="onTileButton"
    >
      <UIcon
        :name="phase !== 'idle' ? busyIcon : opensPanel ? 'lucide:sliders-horizontal' : 'material-symbols:electric-bolt'"
        class="size-4"
      />
      {{ phase !== 'idle' ? busyLabel : opensPanel ? 'CHOOSE' : 'FIRE' }}
      <span
        v-if="phase !== 'idle'"
        class="absolute bottom-0 left-0 h-1 bg-error"
        :style="{ width: `${bar * 100}%` }"
      />
    </button>

    <UModal
      v-if="opensPanel"
      v-model:open="choosing"
      :title="title"
      :ui="{ content: shock ? undefined : VIBRATE }"
      :description="`Up to ${intensity.max}% for ${seconds(duration.max)}s.`"
    >
      <template #body>
        <div class="grid gap-6 [--fader-h:4rem]">
          <div class="grid gap-2.5">
            <div class="flex items-baseline justify-between">
              <span class="font-mono text-xs text-muted uppercase">Power</span>
              <span class="text-3xl font-semibold text-highlighted tabular-nums">{{ power }}<span class="text-[0.6em] opacity-70">%</span></span>
            </div>
            <fader
              v-model="power"
              label="Power"
              color="error"
              :min="intensity.min"
              :max="intensity.max"
              :value-text="`${power}%`"
            />
          </div>
          <div class="grid gap-2.5">
            <div class="flex items-baseline justify-between">
              <span class="font-mono text-xs text-muted uppercase">Time</span>
              <span class="text-3xl font-semibold text-highlighted tabular-nums">{{ seconds(time) }}<span class="text-[0.6em] opacity-70">s</span></span>
            </div>
            <fader
              v-model="time"
              label="Time"
              color="error"
              :min="duration.min"
              :max="duration.max"
              :step="100"
              :value-text="`${seconds(time)} seconds`"
            />
          </div>
        </div>
      </template>
      <template #footer>
        <button
          type="button"
          class="relative flex h-16 w-full cursor-pointer items-center justify-center gap-2 overflow-hidden rounded-field border border-error/55 bg-error/15 text-lg font-semibold tracking-wide text-error disabled:cursor-wait disabled:opacity-80"
          :disabled="phase !== 'idle'"
          @click="fire"
        >
          <UIcon
            :name="phase !== 'idle' ? busyIcon : 'material-symbols:electric-bolt'"
            class="size-5"
          />
          {{ phase !== 'idle' ? busyLabel : 'FIRE' }}
          <span
            v-if="phase !== 'idle'"
            class="absolute bottom-0 left-0 h-1 bg-error"
            :style="{ width: `${bar * 100}%` }"
          />
        </button>
      </template>
    </UModal>
  </control-base>
</template>

<style scoped>
/* While the shock runs, the tile's border and inner glow pulse red. */
.shock-running {
  animation: shock-pulse 0.5s ease-in-out infinite alternate;
}

@keyframes shock-pulse {
  from {
    border-color: color-mix(in oklab, var(--ui-error) 35%, transparent);
    box-shadow: inset 0 0 12px color-mix(in oklab, var(--ui-error) 10%, transparent);
  }

  to {
    border-color: color-mix(in oklab, var(--ui-error) 75%, transparent);
    box-shadow: inset 0 0 30px color-mix(in oklab, var(--ui-error) 28%, transparent);
  }
}
</style>
