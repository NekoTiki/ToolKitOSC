<script setup lang="ts">
// A glass card grouping related fields on a page, with a title, optional help text and actions.
defineProps<{
  title?: string
  description?: string
}>()

defineSlots<{ default(): unknown; actions?(): unknown }>()
</script>

<template>
  <section class="grid min-w-0 grid-cols-1 content-start gap-3.5 rounded-panel glass p-4.5">
    <div
      v-if="title || $slots.actions"
      class="flex flex-wrap items-center gap-2.5"
    >
      <div class="grid min-w-0 flex-1 gap-0.5">
        <h2
          v-if="title"
          class="text-[17px] font-semibold text-highlighted"
        >
          {{ title }}
        </h2>
        <p
          v-if="description"
          class="text-[13px] leading-snug text-muted"
        >
          {{ description }}
        </p>
      </div>
      <slot name="actions" />
    </div>
    <slot />
  </section>
</template>
