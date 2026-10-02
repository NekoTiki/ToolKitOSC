<script setup lang="ts">
import { computed, inject } from 'vue'

import { TILE_BADGES } from './tileBadges'

// The shared tile frame every control renders into: a status light and the name (wrapped to two
// lines, then cut with "…") on top, a middle area (the icon by default), and the control's own
// input pinned to the bottom. Fills whatever cell it's placed in - sizes come from the --tile-*
// variables (see styles/aurora.css), so the user's tile-size setting rescales everything at once.
defineProps<{
  title: string
  icon?: string
  // Switched on: tints the tile and lights the icon in the primary color.
  active?: boolean
  // The status light's color: 'switch' follows `active`, 'value' (sliders, pickers) is always lit
  // in the secondary color, 'danger' is for shockers.
  kind?: 'switch' | 'value' | 'danger'
  // Hide the middle icon and let the bottom input take all the room, for option pickers. Their
  // options then sit at the bottom and scroll if a 2×2 tile still can't fit them all.
  fill?: boolean
}>()

defineSlots<{
  default?(): unknown
  middle?(): unknown
  badge?(): unknown
}>()

const badges = inject(
  TILE_BADGES,
  computed(() => [])
)
</script>

<template>
  <div
    class="relative flex h-full w-full cursor-pointer flex-col gap-2 overflow-hidden rounded-panel border p-(--tile-pad) backdrop-blur-(--aurora-blur) transition-[background-color,border-color,box-shadow] duration-200 select-none"
    :class="
      active
        ? 'border-primary/55 bg-linear-to-b from-primary/20 to-(--aurora-glass) shadow-[inset_0_0_22px_color-mix(in_oklab,var(--ui-primary)_16%,transparent)]'
        : 'border-default bg-(--aurora-glass)'
    "
  >
    <div class="flex min-w-0 shrink-0 items-start gap-2">
      <span
        class="mt-[calc(var(--tile-title)*0.42)] size-2 shrink-0 rounded-full"
        :class="{
          'bg-primary shadow-[0_0_8px_var(--ui-primary)]': kind !== 'value' && kind !== 'danger' && active,
          'bg-(--ui-text-dimmed)': (kind === undefined || kind === 'switch') && !active,
          'bg-secondary shadow-[0_0_6px_var(--ui-secondary)]': kind === 'value',
          'bg-error': kind === 'danger'
        }"
      />
      <span
        class="line-clamp-2 min-w-0 flex-1 text-(length:--tile-title) leading-tight font-medium break-words text-highlighted"
        :title="title"
      >{{ title }}</span>
      <slot name="badge" />
      <span
        v-for="badge in badges"
        :key="badge.key"
        class="grid size-5 shrink-0 place-items-center overflow-hidden rounded-full bg-(--aurora-well)"
        :class="badge.tone === 'warning' ? 'text-warning' : 'text-muted'"
        :title="badge.label"
      >
        <img
          v-if="badge.avatar"
          :src="badge.avatar"
          :alt="badge.label"
          class="size-full object-cover"
        >
        <UIcon
          v-else-if="badge.icon"
          :name="badge.icon"
          class="size-3"
        />
      </span>
    </div>

    <div
      v-if="!fill"
      class="grid min-h-0 min-w-0 flex-1 place-items-center text-center"
    >
      <slot name="middle">
        <UIcon
          v-if="icon"
          :name="icon"
          class="size-(--tile-icon) transition-colors duration-200"
          :class="active ? 'text-primary drop-shadow-[0_0_10px_color-mix(in_oklab,var(--ui-primary)_55%,transparent)]' : 'text-dimmed'"
        />
      </slot>
    </div>

    <div
      class="min-w-0"
      :class="fill ? 'flex min-h-0 flex-1 flex-col justify-end' : 'shrink-0'"
    >
      <slot />
    </div>
  </div>
</template>
