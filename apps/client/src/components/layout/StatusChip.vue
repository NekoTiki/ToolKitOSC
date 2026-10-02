<script setup lang="ts">
// A top-bar chip: a click target (never hover-only - VR pointers can't hover reliably) showing a
// state dot or icon plus a label. The label drops below `lg` so the bar still fits an overlay-sized
// window; `title` keeps it discoverable there.
export type StatusTone = 'success' | 'warning' | 'error' | 'muted'

withDefaults(
  defineProps<{
    label?: string
    icon?: string
    tone?: StatusTone
    // Degraded but not down (e.g. the server can't show some control types) - a small corner dot
    // rather than recoloring the whole chip.
    warning?: boolean
  }>(),
  { tone: 'muted', label: undefined, icon: undefined }
)

const DOT: Record<StatusTone, string> = {
  success: 'bg-success shadow-[0_0_6px_var(--ui-success)]',
  warning: 'bg-warning shadow-[0_0_6px_var(--ui-warning)]',
  error: 'bg-error/70',
  muted: 'bg-(--ui-text-dimmed)'
}

const ICON: Record<StatusTone, string> = {
  success: 'text-success',
  warning: 'text-warning',
  error: 'text-error',
  muted: 'text-muted'
}

defineSlots<{ default?(): unknown }>()
</script>

<template>
  <button
    type="button"
    class="relative flex h-10.5 shrink-0 cursor-pointer items-center gap-2 rounded-2xl glass px-3 text-sm font-medium text-default transition-colors hover:bg-elevated"
    :title="label"
  >
    <UIcon
      v-if="icon"
      :name="icon"
      class="size-4.5"
      :class="ICON[tone]"
    />
    <span
      v-else
      class="size-2 rounded-full"
      :class="DOT[tone]"
    />
    <span
      v-if="label"
      class="max-w-40 truncate max-lg:hidden"
    >{{ label }}</span>
    <slot />
    <span
      v-if="warning"
      class="absolute -top-0.5 -right-0.5 size-2.5 rounded-full bg-warning ring-2 ring-(--ui-bg)"
    />
  </button>
</template>
