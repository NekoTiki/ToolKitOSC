<script setup lang="ts">
defineProps<{ title: string; icon?: string }>()
</script>

<template>
  <div
    class="bg-default ring-default grid aspect-square cursor-pointer grid-rows-[1fr_auto] overflow-hidden rounded-lg ring select-none"
  >
    <div class="relative flex items-center justify-center text-center text-2xl">
      {{ title }}
      <UIcon
        v-if="icon"
        :name="icon"
        class="absolute size-32 opacity-10"
      />
    </div>
    <div class="p-2">
      <slot />
    </div>
  </div>
</template>

<style scoped></style>
