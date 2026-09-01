<script setup lang="ts">
import _ from 'lodash'
import { computed, ref } from 'vue'

import { useOscMessages } from '../composables/useOscMessages'

const { get } = useOscMessages()

const props = defineProps<{
  title: string
  icon?: string
  address: string
}>()

const emit = defineEmits<{ (e: 'update:value', value: number): void }>()

const sliderValue = computed({
  get: () => {
    return (get<number>(props.address, 0) || 0) * 100
  },
  set: _.debounce(
    (newValue: number) => {
      if (newValue === sliderValue.value) return

      emit('update:value', newValue / 100)
    },
    25,
    { leading: true, trailing: false, maxWait: 25 }
  )
})

const isDragging = ref(false)
const sliderRef = ref<HTMLElement>()

const rotationAngle = computed(() => {
  return (sliderValue.value / 100) * 360 - 90
})

const handleMouseDown = (event: MouseEvent): void => {
  event.preventDefault()
  isDragging.value = true
  updateSliderValue(event)

  const handleMouseMove = (e: MouseEvent): void => {
    if (isDragging.value) {
      updateSliderValue(e)
    }
  }

  const handleMouseUp = (): void => {
    isDragging.value = false
    document.removeEventListener('mousemove', handleMouseMove)
    document.removeEventListener('mouseup', handleMouseUp)
  }

  document.addEventListener('mousemove', handleMouseMove)
  document.addEventListener('mouseup', handleMouseUp)
}

const updateSliderValue = (event: MouseEvent): void => {
  if (!sliderRef.value) return

  const rect = sliderRef.value.getBoundingClientRect()

  const centerX = rect.left + rect.width / 2
  const centerY = rect.top + rect.height / 2

  const deltaX = event.clientX - centerX
  const deltaY = event.clientY - centerY

  let angle = Math.atan2(deltaY, deltaX) * (180 / Math.PI)

  angle = angle + 90
  if (angle < 0) angle += 360
  if (angle >= 360) angle -= 360

  const percentage = (angle / 360) * 100

  sliderValue.value = Math.round(Math.max(0, Math.min(100, percentage)))
}
</script>

<template>
  <div
    ref="sliderRef"
    class="bg-default ring-default relative aspect-square cursor-pointer overflow-hidden rounded-lg ring select-none"
    @mousedown="handleMouseDown"
  >
    <svg
      class="absolute inset-0 z-10 h-full w-full"
      viewBox="0 0 100 100"
    >
      <!-- Progress track background (gray, full circle) -->
      <circle
        cx="50"
        cy="50"
        r="35"
        fill="none"
        stroke="currentColor"
        stroke-width="10"
        class="text-primary/20"
        stroke-linecap="round"
        stroke-dasharray="219.9 219.9"
        stroke-dashoffset="0"
        transform="rotate(-90 50 50)"
      />

      <!-- Progress track (colored, value) -->
      <circle
        cx="50"
        cy="50"
        r="35"
        fill="none"
        stroke="currentColor"
        stroke-width="12"
        class="text-primary"
        stroke-linecap="round"
        :stroke-dasharray="`${(sliderValue / 100) * 219.9} 219.9`"
        stroke-dashoffset="0"
        transform="rotate(-90 50 50)"
      />
    </svg>

    <div class="pointer-events-none absolute inset-0 flex flex-col items-center justify-center">
      <div class="text-2xl font-medium">
        {{ title }}
      </div>
      <div class="text-sm font-bold opacity-70">
        {{ sliderValue.toFixed() }}%
      </div>
      <UIcon
        v-if="icon"
        :name="icon"
        class="absolute size-32 opacity-10"
      />
    </div>

    <div
      class="bg-primary pointer-events-none absolute h-12 w-12 rounded-full shadow-xl/50"
      :style="{
        left: `${50 + 35 * Math.cos(((rotationAngle + 0) * Math.PI) / 180)}%`,
        top: `${50 + 35 * Math.sin(((rotationAngle + 0) * Math.PI) / 180)}%`,
        transform: 'translate(-50%, -50%)'
      }"
    />
  </div>
</template>

<style scoped></style>
