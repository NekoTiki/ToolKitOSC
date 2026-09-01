<script setup lang="ts">
import ControlBase from './ControlBase.vue'

const state = defineModel<boolean>()

defineProps<{ title: string; icon?: string }>()
</script>

<template>
  <control-base
    :title="title"
    :icon="icon"
    @click="state = !state"
  >
    <div class="ring-default rounded-md p-2 ring">
      <div
        class="h-6 w-1/2 rounded-md transition-all"
        :class="{
          'bg-primary translate-x-full': state,
          'bg-inverted': !state
        }"
      />
    </div>
  </control-base>
</template>

<style scoped></style>
