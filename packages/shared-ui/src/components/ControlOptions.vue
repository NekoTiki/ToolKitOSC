<script setup lang="ts">
// The option buttons shared by every picker tile (Enum, Toggle Logic, Toy Pattern). All options
// stay visible and tappable at once instead of hiding behind a dropdown - a dropdown is a poor fit
// for a VR pointer. Up to four per row; a 2×2 tile (see utils/tiles.ts) scrolls when even that
// isn't enough room. Long labels are cut with "…" and shown in full on hover.
export interface ControlOption {
  key: string | number
  name: string
  icon?: string
  selected: boolean
  // Blocked by the host's active profile: shown, but can't be picked.
  disabled?: boolean
}

const props = defineProps<{
  options: ControlOption[]
  // Pill-shaped buttons, for pickers whose options are modes rather than values (patterns).
  pill?: boolean
}>()

const emit = defineEmits<{ (e: 'select', index: number): void }>()

const columns = Math.min(4, Math.max(1, props.options.length))
</script>

<template>
  <div
    class="grid max-h-full auto-rows-(--tile-option) content-end gap-1.5 overflow-y-auto"
    :style="{ gridTemplateColumns: `repeat(${columns}, minmax(0, 1fr))` }"
    role="radiogroup"
  >
    <button
      v-for="(option, index) in options"
      :key="option.key"
      type="button"
      role="radio"
      :aria-checked="option.selected"
      :aria-disabled="option.disabled || undefined"
      :title="option.disabled ? `${option.name} (blocked by the streamer)` : option.name"
      class="flex h-(--tile-option) min-w-0 items-center justify-center gap-1.5 border px-2 text-[calc(var(--tile-title)*0.82)] transition-colors"
      :class="[
        pill ? 'rounded-full' : 'rounded-field',
        option.selected
          ? 'border-secondary bg-secondary font-semibold text-(--ui-color-secondary-950)'
          : option.disabled
            ? 'border-dashed border-default bg-(--aurora-well) text-dimmed'
            : 'border-default bg-(--aurora-well) text-muted hover:text-default',
        option.disabled ? 'cursor-not-allowed opacity-60' : 'cursor-pointer'
      ]"
      @click.stop="!option.disabled && emit('select', index)"
    >
      <UIcon
        v-if="option.disabled"
        name="i-lucide-lock"
        class="size-3.5 shrink-0"
      />
      <UIcon
        v-else-if="option.icon"
        :name="option.icon"
        class="size-4 shrink-0"
      />
      <span class="truncate">{{ option.name }}</span>
    </button>
  </div>
</template>
