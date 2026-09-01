<script setup lang="ts">
import { ref, watch } from 'vue'

import { useOpenShockControl } from '../composables/useOpenShockControl'
import type { OpenShockControl } from '../types/controls'

const props = defineProps<{
  controlId: string
  title: string
  mode: OpenShockControl['mode']
  intensity: OpenShockControl['intensity']
  duration: OpenShockControl['duration']
}>()

defineEmits<{ (e: 'run'): void }>()

const { lastUpdate } = useOpenShockControl()

const powerValue = ref(0)
const timeValue = ref(0)
const coolDown = ref(0)
let timeoutId = null as ReturnType<typeof setTimeout> | null
let cooldownTimeoutId = null as ReturnType<typeof setTimeout> | null
let animating = false
let cooldownEndTime = 0

const startRandomizing = (duration: number, targetPower?: number, targetTime?: number): void => {
  if (animating) return
  animating = true

  const finalPowerValue =
    typeof targetPower === 'number' ? targetPower : Math.floor(Math.random() * props.intensity.max)
  const finalTimeValue =
    typeof targetTime === 'number' ? targetTime : Math.floor(Math.random() * props.duration.max)
  const startTime = Date.now()

  if (timeoutId) clearTimeout(timeoutId)

  const getRandomValue = (min: number, max: number): number =>
    Math.floor(Math.random() * (max - min + 1)) + min

  const animate = (): void => {
    const elapsed = Date.now() - startTime
    const progress = Math.min(elapsed / duration, 1)
    const eased = 1 - Math.pow(1 - progress, 3)

    const randomPowerPart = Math.floor(getRandomValue(0, props.intensity.max) * (1 - eased))
    const randomTimePart = Math.floor(getRandomValue(0, props.duration.max) * (1 - eased))

    const tweenPowerPart = Math.floor(finalPowerValue * eased)
    const tweenTimePart = Math.floor(finalTimeValue * eased)

    powerValue.value = randomPowerPart + tweenPowerPart
    timeValue.value = randomTimePart + tweenTimePart

    if (progress >= 1) {
      timeoutId = null
      animating = false
      powerValue.value = finalPowerValue
      timeValue.value = finalTimeValue

      startCooldown()

      return
    }

    const delay = 30 + Math.floor(progress * progress * 200)
    timeoutId = setTimeout(animate, delay)
  }

  animate()
}

const startCooldown = (): void => {
  if (cooldownTimeoutId) clearTimeout(cooldownTimeoutId)

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
</script>

<template>
  <UCard
    :ui="{ root: 'h-[180px] sm:h-[220px]', body: 'h-full relative isolate p-2 sm:p-2' }"
    @click="$emit('run')"
  >
    <div class="flex h-full flex-col items-center justify-center gap-2 select-none">
      <div class="text-2xl">
        {{ title }}
      </div>
      <div class="relative min-h-0 flex-grow cursor-pointer">
        <svg
          viewBox="0 0 400 400"
          xmlns="http://www.w3.org/2000/svg"
          class="h-full w-full transition-transform duration-300"
          :class="{ 'animate-spin': animating }"
        >
          <circle
            cx="200"
            cy="200"
            r="198"
            fill="white"
            class="fill-[var(--ui-border)]"
          />

          <g transform="translate(200,200)">
            <path
              d="M0 0 L190 0 A190 190 0 0 1 134.35 134.35 Z"
              class="fill-primary"
            />
            <path
              d="M0 0 L134.35 134.35 A190 190 0 0 1 0 190 Z"
              class="fill-[var(--ui-bg)]"
            />
            <path
              d="M0 0 L0 190 A190 190 0 0 1 -134.35 134.35 Z"
              class="fill-primary"
            />
            <path
              d="M0 0 L-134.35 134.35 A190 190 0 0 1 -190 0 Z"
              class="fill-[var(--ui-bg)]"
            />
            <path
              d="M0 0 L-190 0 A190 190 0 0 1 -134.35 -134.35 Z"
              class="fill-primary"
            />
            <path
              d="M0 0 L-134.35 -134.35 A190 190 0 0 1 0 -190 Z"
              class="fill-[var(--ui-bg)]"
            />
            <path
              d="M0 0 L0 -190 A190 190 0 0 1 134.35 -134.35 Z"
              class="fill-primary"
            />
            <path
              d="M0 0 L134.35 -134.35 A190 190 0 0 1 190 0 Z"
              class="fill-[var(--ui-bg)]"
            />
          </g>
        </svg>
        <div class="absolute inset-0 flex flex-col items-center justify-center">
          <div class="text-5xl font-bold text-white drop-shadow-[0px_0px_3px_rgba(0,0,0,1)]">
            {{ powerValue }}
          </div>
          <div class="text-xl text-white drop-shadow-[0px_0px_3px_rgba(0,0,0,1)]">
            {{ (timeValue / 1000).toFixed(1) }}s
          </div>
        </div>
      </div>
    </div>
    <UTooltip class="absolute top-3 left-3 z-20 cursor-grab rounded-lg">
      <UIcon
        v-if="mode === 'Shock'"
        name="material-symbols:electric-bolt"
        class="size-6"
      />
      <UIcon
        v-else
        name="lucide:waves"
        class="size-6"
      />

      <template #content>
        {{ mode }}
      </template>
    </UTooltip>
    <div
      v-if="coolDown > 0"
      class="absolute inset-0 z-20 flex h-full w-full cursor-pointer flex-col items-center justify-center rounded-lg bg-black/70 p-6 font-bold text-white opacity-0 transition-opacity duration-300 hover:opacity-100"
    >
      <span class="text-3xl">{{ coolDown.toFixed(1) }}s</span>
      <span>Cooldown</span>
    </div>
  </UCard>
</template>

<style scoped></style>
