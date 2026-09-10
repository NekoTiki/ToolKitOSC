<script setup lang="ts">
import ControlBase from './ControlBase.vue'

const state = defineModel<boolean>()

defineProps<{ title: string; icon?: string }>()
</script>

<template>
  <control-base
    :title="title"
    :icon="icon"
    :active="state"
    @click="state = !state"
  >
    <!-- Full custom, not a UI-kit switch: a full-width Off/On capsule, not a small centered knob -
    both labels are always on screen (so there's no ambiguity about which state means what), and a
    solid color panel slides to sit behind whichever one is active. Reads at a glance from across a
    room, which matters more here than it would in a mouse-driven form. The whole card is already
    the click target (see @click above), so nothing in here needs its own listener. The sliding
    panel's width/offset are both expressed as this element's own padding (0.25rem, i.e. p-1) plus
    a percentage, not a value tied to any assumed container size, so it lines up correctly at
    whatever width this ends up rendered at. -->
    <div
      class="ring-default relative flex h-11 w-full items-center overflow-hidden rounded-full p-1 ring"
    >
      <div
        class="bg-primary absolute inset-y-1 left-1 w-[calc(50%-0.25rem)] rounded-full shadow-md transition-transform"
        :class="{ 'translate-x-[calc(100%+0.25rem)]': state }"
      />
      <div
        class="relative z-10 flex flex-1 items-center justify-center gap-1.5 text-sm font-bold transition-colors"
        :class="state ? 'text-muted' : 'text-inverted'"
      >
        <UIcon
          name="i-lucide-x"
          class="size-4"
        />
        Off
      </div>
      <div
        class="relative z-10 flex flex-1 items-center justify-center gap-1.5 text-sm font-bold transition-colors"
        :class="state ? 'text-inverted' : 'text-muted'"
      >
        <UIcon
          name="i-lucide-check"
          class="size-4"
        />
        On
      </div>
    </div>
  </control-base>
</template>

<style scoped></style>
