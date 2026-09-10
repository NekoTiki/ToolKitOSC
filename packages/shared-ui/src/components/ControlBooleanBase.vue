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
    <!-- The whole card is the click target (see @click above) - a real USwitch here would toggle
    itself independently and double up with that, so it's pointer-events-none and purely a themed,
    on/off-labeled visual readout of `state`, consistent with the rest of the app's switches. -->
    <div class="flex items-center justify-center">
      <USwitch
        :model-value="state"
        size="xl"
        checked-icon="i-lucide-check"
        unchecked-icon="i-lucide-x"
        class="pointer-events-none"
      />
    </div>
  </control-base>
</template>

<style scoped></style>
