<script setup lang="ts">
import ControlBase from './ControlBase.vue'

const state = defineModel<boolean>()

defineProps<{ title: string; icon?: string }>()

const toggle = (): void => {
  state.value = !state.value
}
</script>

<template>
  <!-- The whole tile is the switch (a big target for a VR pointer), so it also carries the switch
  role and keyboard handling. -->
  <control-base
    :title="title"
    :icon="icon"
    :active="state"
    role="switch"
    tabindex="0"
    :aria-checked="!!state"
    :aria-label="title"
    @click="toggle"
    @keydown.enter.space.prevent="toggle"
  >
    <!-- A full-width OFF/ON capsule rather than a small knob: both labels are always on screen, so
    there's no doubt which state means what, and a colored panel slides behind the active one.
    Reads at a glance from across a room. The panel's width/offset are expressed from this
    element's own padding (p-1) plus a percentage, so it lines up at any tile size. -->
    <div class="relative flex h-(--tile-control) w-full items-center overflow-hidden rounded-field well p-1 text-[calc(var(--tile-title)*0.95)] font-semibold tracking-wide">
      <div
        class="absolute inset-y-1 left-1 w-[calc(50%-0.25rem)] rounded-[calc(var(--radius-field)-0.25rem)] transition-[translate,background-color] duration-200"
        :class="state ? 'translate-x-full bg-primary shadow-[0_0_16px_color-mix(in_oklab,var(--ui-primary)_45%,transparent)]' : 'bg-accented'"
      />
      <span
        class="relative z-10 flex flex-1 items-center justify-center transition-colors"
        :class="state ? 'text-dimmed' : 'text-highlighted'"
      >OFF</span>
      <span
        class="relative z-10 flex flex-1 items-center justify-center transition-colors"
        :class="state ? 'text-inverted' : 'text-dimmed'"
      >ON</span>
    </div>
  </control-base>
</template>
