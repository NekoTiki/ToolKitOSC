<script setup lang="ts">
// A centered "nothing here" message: icon, title, explanation, optional actions.
defineProps<{
  icon: string
  title: string
  description?: string
}>()

defineSlots<{ default?(): unknown; actions?(): unknown }>()
</script>

<template>
  <div class="mx-auto grid max-w-lg justify-items-center gap-2.5 px-5 py-12 text-center text-muted">
    <UIcon
      :name="icon"
      class="size-10"
    />
    <h2 class="text-[22px] font-semibold text-highlighted">
      {{ title }}
    </h2>
    <p
      v-if="description"
      class="leading-relaxed"
    >
      {{ description }}
    </p>
    <slot />
    <div
      v-if="$slots.actions"
      class="mt-2 flex flex-wrap justify-center gap-2"
    >
      <slot name="actions" />
    </div>
  </div>
</template>
