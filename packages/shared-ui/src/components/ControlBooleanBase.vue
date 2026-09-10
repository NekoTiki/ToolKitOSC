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
    <!-- Full custom, not a UI-kit switch: a big, unambiguous pill-and-knob shape - track color and
    knob position both flip, and the icon repeats the state a third way - so the toggle reads at a
    glance from across a room, which matters more here than it would in a mouse-driven form. The
    whole card is already the click target (see @click above), so nothing here needs its own
    listener - the knob's width is exactly half the track's inner width (see w-1/2/translate-x-full
    below), which is what makes it a circle and what makes sliding it by its own width land it
    exactly on the other side, at any size this ends up rendered at. -->
    <div class="flex items-center justify-center">
      <div
        class="ring-default flex h-10 w-20 items-center rounded-full p-1 ring transition-colors"
        :class="state ? 'bg-primary' : 'bg-elevated'"
      >
        <div
          class="bg-default flex h-full w-1/2 items-center justify-center rounded-full shadow-md transition-transform"
          :class="{ 'translate-x-full': state }"
        >
          <UIcon
            :name="state ? 'i-lucide-check' : 'i-lucide-x'"
            class="size-5"
            :class="state ? 'text-primary' : 'text-muted'"
          />
        </div>
      </div>
    </div>
  </control-base>
</template>

<style scoped></style>
