<script setup lang="ts">
const model = defineModel<boolean>({ required: true })
</script>

<template>
  <label class="mt-2 flex cursor-pointer items-center justify-end gap-2">
    <span class="text-sm text-muted">Show raw address list</span>
    <USwitch v-model="model" />
  </label>
</template>
