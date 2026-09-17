<script setup lang="ts">
// Built on ControlBase (title/icon card) like every other control. The whole card is the click
// target (ControlBase forwards an unhandled `@click` straight onto its root element, same as
// ControlBooleanBase's `@click="state = !state"` above it does), same as every other control in
// this app - the UButton inside is just a real, obvious visual affordance for that action (a
// one-shot "Apply" rather than a toggle/value to show), not a separate interactive target of its
// own, hence `.stop` on it: without it, a click on the button would also bubble up to the card's
// own handler and fire `run` twice.
import ControlBase from './ControlBase.vue'

defineProps<{ title: string; icon?: string }>()
const emit = defineEmits<{ (e: 'run'): void }>()
</script>

<template>
  <control-base
    :title="title"
    :icon="icon"
    @click="emit('run')"
  >
    <UButton
      label="Apply"
      icon="i-lucide-play"
      color="primary"
      block
      @click.stop="emit('run')"
    />
  </control-base>
</template>

<style scoped></style>
