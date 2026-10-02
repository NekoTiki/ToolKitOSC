<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from 'vue'

import { useOpenShockControl } from '../composables/useOpenShockControl'
import type { OpenShockControl } from '../types/controls'
import ControlBase from './ControlBase.vue'

const props = defineProps<{
  controlId: string
  title: string
  mode: OpenShockControl['mode']
  intensity: OpenShockControl['intensity']
  duration: OpenShockControl['duration']
}>()

const emit = defineEmits<{ (e: 'run'): void }>()

const { lastUpdate } = useOpenShockControl()

// The gamble: a press rolls a random intensity and duration (picked by the host, see the client's
// useControls.ts), and the shock is only sent once the roll ends - so the roll is also the
// countdown. Two slot-machine reels spin through the control's range, slow down and land on the
// result; then the tile flashes and the cooldown drains on the button.
const powerValue = ref(0)
const timeValue = ref(0)
const coolDown = ref(0)
let timeoutId = null as ReturnType<typeof setTimeout> | null
let cooldownTimeoutId = null as ReturnType<typeof setTimeout> | null
let hitTimeoutId = null as ReturnType<typeof setTimeout> | null
let cooldownEndTime = 0
// Whether anything has been rolled yet - until then the reels show their range.
const hasResult = ref(false)
const rolling = ref(false)
// 0-1 through the current roll, for the button's progress bar.
const rollProgress = ref(0)
// Briefly true when the reels land, for the "hit" flash.
const hit = ref(false)
// The cooldown's full length, captured when it starts, so the bar can show progress through it.
const coolDownTotal = ref(0)

const randomBetween = (min: number, max: number): number => Math.floor(Math.random() * (max - min + 1)) + min

const startRandomizing = (duration: number, targetPower?: number, targetTime?: number): void => {
  if (rolling.value) return

  const finalPowerValue = targetPower ?? randomBetween(props.intensity.min, props.intensity.max)
  const finalTimeValue = targetTime ?? randomBetween(props.duration.min, props.duration.max)
  const startTime = Date.now()

  if (timeoutId) clearTimeout(timeoutId)
  if (hitTimeoutId) clearTimeout(hitTimeoutId)

  rolling.value = true
  hasResult.value = true
  hit.value = false

  const animate = (): void => {
    const progress = Math.min((Date.now() - startTime) / Math.max(duration, 1), 1)
    // Ease out: the reels drift from random values toward the result as they slow down.
    const eased = 1 - Math.pow(1 - progress, 3)

    rollProgress.value = progress
    powerValue.value = Math.round(randomBetween(props.intensity.min, props.intensity.max) * (1 - eased) + finalPowerValue * eased)
    timeValue.value = Math.round(randomBetween(props.duration.min, props.duration.max) * (1 - eased) + finalTimeValue * eased)

    if (progress >= 1) {
      timeoutId = null
      rolling.value = false
      powerValue.value = finalPowerValue
      timeValue.value = finalTimeValue

      hit.value = true
      hitTimeoutId = setTimeout(() => (hit.value = false), 700)

      startCooldown()

      return
    }

    // Ticks get further apart as the roll goes on, like reels slowing down.
    timeoutId = setTimeout(animate, 40 + Math.floor(progress * progress * 220))
  }

  animate()
}

const startCooldown = (): void => {
  if (cooldownTimeoutId) clearTimeout(cooldownTimeoutId)

  coolDownTotal.value = Math.max(0, (cooldownEndTime - Date.now()) / 1000)

  const updateCooldown = (): void => {
    const now = Date.now()
    const remaining = Math.max(0, (cooldownEndTime - now) / 1000)
    coolDown.value = remaining

    if (remaining > 0) {
      cooldownTimeoutId = setTimeout(updateCooldown, 100)
    }
  }

  updateCooldown()
}

watch(
  () => lastUpdate.value,
  (newValue) => {
    if (!newValue || newValue.controlId !== props.controlId) return

    const { animationDuration, duration, intensity, cooldownEnd } = newValue.value

    cooldownEndTime = cooldownEnd

    startRandomizing(animationDuration, intensity, duration)
  },
  { deep: true }
)

onBeforeUnmount(() => {
  for (const id of [timeoutId, cooldownTimeoutId, hitTimeoutId]) if (id) clearTimeout(id)
})

const seconds = (ms: number): string => (ms / 1000).toFixed(1)
// For the range shown before a roll, where space is tight: 2 instead of 2.0.
const shortSeconds = (ms: number): string => String(Number((ms / 1000).toFixed(1)))

// Where a value sits within its configured range, 0-1, for the gauge under each reel.
const position = (value: number, range: { min: number; max: number }): number =>
  range.max > range.min ? Math.min(1, Math.max(0, (value - range.min) / (range.max - range.min))) : 1

const reels = computed(() => [
  {
    label: 'Power',
    range: `${props.intensity.min}–${props.intensity.max}`,
    value: String(powerValue.value),
    unit: '%',
    position: position(powerValue.value, props.intensity)
  },
  {
    label: 'Time',
    range: `${shortSeconds(props.duration.min)}–${shortSeconds(props.duration.max)}`,
    value: seconds(timeValue.value),
    unit: 's',
    position: position(timeValue.value, props.duration)
  }
])

const coolingDown = computed(() => coolDown.value > 0)

const fire = (): void => {
  if (!coolingDown.value && !rolling.value) emit('run')
}
</script>

<template>
  <control-base
    :title="title"
    kind="danger"
    :class="{ 'shock-rolling': rolling, 'shock-hit': hit }"
  >
    <template #badge>
      <span
        class="grid size-5 shrink-0 place-items-center rounded-full bg-error/15 text-error"
        :title="mode"
      >
        <UIcon
          :name="mode === 'Shock' ? 'material-symbols:electric-bolt' : 'lucide:waves'"
          class="size-3"
        />
      </span>
    </template>

    <template #middle>
      <div class="grid w-full grid-cols-2 gap-1.5">
        <div
          v-for="reel in reels"
          :key="reel.label"
          class="relative grid min-w-0 gap-1 overflow-hidden rounded-field well px-2 pt-[calc(var(--tile-pad)*0.45)] pb-[calc(var(--tile-pad)*0.8)] transition-colors duration-300"
          :class="{ 'border-error/60': rolling || hit }"
        >
          <span class="font-mono text-[10px] leading-none text-muted uppercase">{{ reel.label }}</span>
          <span
            class="truncate text-[calc(var(--tile-value)*0.78)] leading-none font-semibold tabular-nums"
            :class="[rolling ? 'reel-spin text-error' : 'text-highlighted', { 'reel-hit': hit }]"
          >
            <template v-if="hasResult">{{ reel.value }}<span class="text-[0.55em] opacity-70">{{ reel.unit }}</span></template>
            <!-- Before the first roll: what's at stake, the range the reel can land in. -->
            <span
              v-else
              class="text-[calc(var(--tile-value)*0.42)] whitespace-nowrap text-muted"
            >{{ reel.range }}{{ reel.unit }}</span>
          </span>
          <!-- Where the roll sits in the configured range: the minimum on the left, the maximum on
          the right. -->
          <span
            v-if="hasResult"
            class="absolute bottom-[calc(var(--tile-pad)*0.3)] left-2 h-1 rounded-full bg-error/80"
            :style="{ width: `calc((100% - 1rem) * ${reel.position})` }"
          />
        </div>
      </div>
    </template>

    <!-- The whole tile isn't a trigger here (unlike toggles): a shock should take a deliberate
    press on the button, not a stray pointer click anywhere on the tile. -->
    <button
      type="button"
      class="relative flex h-(--tile-control) w-full cursor-pointer items-center justify-center gap-2 overflow-hidden rounded-field border border-error/55 bg-error/15 text-[calc(var(--tile-title)*0.95)] font-semibold tracking-wide text-error transition-opacity disabled:cursor-wait"
      :class="{ 'disabled:opacity-60': !rolling }"
      :disabled="coolingDown || rolling"
      @click.stop="fire"
    >
      <UIcon
        :name="rolling ? 'lucide:dices' : coolingDown ? 'lucide:hourglass' : 'material-symbols:electric-bolt'"
        class="size-4"
        :class="{ 'animate-spin': rolling }"
      />
      <!-- Just the seconds while cooling down: "COOLDOWN 3.0s" wrapped on the smallest tiles, and
      the hourglass plus the draining bar already say what it is. -->
      {{ rolling ? 'ROLLING…' : coolingDown ? `${coolDown.toFixed(1)}s` : 'FIRE' }}
      <span
        v-if="rolling"
        class="absolute bottom-0 left-0 h-1 bg-error"
        :style="{ width: `${rollProgress * 100}%` }"
      />
      <span
        v-else-if="coolingDown && coolDownTotal > 0"
        class="absolute bottom-0 left-0 h-1 bg-error"
        :style="{ width: `${(coolDown / coolDownTotal) * 100}%` }"
      />
    </button>
  </control-base>
</template>

<style scoped>
/* While rolling, the tile's border and inner glow pulse red; on landing it flashes once. */
.shock-rolling {
  animation: shock-pulse 0.6s ease-in-out infinite alternate;
}

.shock-hit {
  animation: shock-flash 0.7s ease-out;
}

/* The reel digits shake vertically with a motion blur, like a spinning slot reel. */
.reel-spin {
  animation: reel-spin 0.12s linear infinite;
}

.reel-hit {
  animation: reel-hit 0.45s cubic-bezier(0.2, 1.6, 0.4, 1);
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

@keyframes shock-flash {
  0% {
    border-color: var(--ui-error);
    background-color: color-mix(in oklab, var(--ui-error) 35%, transparent);
    box-shadow: 0 0 28px color-mix(in oklab, var(--ui-error) 55%, transparent);
  }

  100% {
    box-shadow: 0 0 0 transparent;
  }
}

@keyframes reel-spin {
  0% {
    transform: translateY(-6%);
    text-shadow: 0 6px 4px color-mix(in oklab, var(--ui-error) 45%, transparent);
  }

  50% {
    transform: translateY(6%);
    text-shadow: 0 -6px 4px color-mix(in oklab, var(--ui-error) 45%, transparent);
  }

  100% {
    transform: translateY(-6%);
  }
}

@keyframes reel-hit {
  0% {
    transform: scale(1.35);
    color: #fff;
  }

  100% {
    transform: scale(1);
  }
}
</style>
