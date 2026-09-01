<script setup lang="ts">
const model = defineModel<boolean>({ required: true })
</script>

<template>
  <div class="-mt-1 flex items-center justify-end gap-2">
    <span class="text-sm text-neutral-500">Show raw address list</span>
    <USwitch v-model="model" />
  </div>
</template>

<style scoped></style>
