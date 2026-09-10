<script setup lang="ts">
// `active` is optional and purely cosmetic (tints the whole card) - only a boolean-flavored
// control (see ControlBooleanBase.vue) currently passes it, so every other consumer (Enum,
// BooleanEnum) renders exactly as before.
defineProps<{ title: string; icon?: string; active?: boolean }>()
</script>

<template>
  <div
    class="grid aspect-square cursor-pointer grid-rows-[1fr_auto] overflow-hidden rounded-lg ring transition-colors select-none"
    :class="active ? 'bg-primary/10 ring-primary' : 'bg-default ring-default'"
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
