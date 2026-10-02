<script setup lang="ts">
import { ref } from 'vue'

import ControlBase from './ControlBase.vue'

defineProps<{ title: string; icon?: string }>()
const emit = defineEmits<{ (e: 'run'): void }>()

// A brief lit state after each tap - applying a preset has no lasting on/off state of its own to
// show, so this is the only confirmation the tap landed.
const flashing = ref(false)
let flashTimeout: ReturnType<typeof setTimeout> | undefined

const run = (): void => {
  emit('run')
  flashing.value = true
  clearTimeout(flashTimeout)
  flashTimeout = setTimeout(() => (flashing.value = false), 700)
}
</script>

<template>
  <control-base
    :title="title"
    :icon="icon ?? 'i-lucide-play'"
    :active="flashing"
    role="button"
    tabindex="0"
    :aria-label="`Apply ${title}`"
    @click="run"
    @keydown.enter.space.prevent="run"
  >
    <div
      class="flex h-(--tile-control) items-center justify-center gap-2 rounded-field border text-[calc(var(--tile-title)*0.95)] font-semibold tracking-wide transition-colors"
      :class="flashing ? 'border-primary bg-primary text-inverted' : 'border-primary/50 text-primary'"
    >
      <UIcon
        name="i-lucide-play"
        class="size-4"
      />
      APPLY
    </div>
  </control-base>
</template>
