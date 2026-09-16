<script setup lang="ts">
// `active` is optional and purely cosmetic (tints the whole card) - only a boolean-flavored
// control (see ControlBooleanBase.vue) currently passes it, so every other consumer (Enum,
// BooleanEnum) renders exactly as before.
defineProps<{ title: string; icon?: string; active?: boolean }>()
</script>

<template>
  <!-- grid-cols-[minmax(0,1fr)]: without an explicit column, this grid's single implicit column
  sizes itself to its content (same automatic-minimum-size behavior as flex, just one layer up) -
  it'll happily grow past this card's own width to fit an untruncated title, which then only gets
  a hard clip from `overflow-hidden` below with no "..." to show for it, instead of ever actually
  truncating within its own row. Pinning the column to the box's own width (not its content's)
  is what lets the title row's `min-w-0`/`truncate` below do anything at all. -->
  <div
    class="grid aspect-square cursor-pointer grid-cols-[minmax(0,1fr)] grid-rows-[minmax(0,1fr)_auto] overflow-hidden rounded-lg ring transition-colors select-none"
    :class="active ? 'bg-primary/10 ring-primary' : 'bg-default ring-default'"
  >
    <div class="relative flex min-w-0 items-center justify-center px-2 text-center text-2xl">
      <!-- min-w-0 (both here and on the row above): a flex/grid child otherwise refuses to shrink
      past its own content's intrinsic width (the "min-width: auto" default), which would silently
      stop `truncate` from ever clipping anything, no matter how long `title` gets. -->
      <span class="w-full min-w-0 truncate">{{ title }}</span>
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
