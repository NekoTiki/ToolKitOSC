<script setup lang="ts">
// A large selectable card (control type, AI profile, who can use your controls...): icon, title,
// one-line description, and when disabled, the reason why - so an unavailable option explains
// itself instead of just being greyed out.
defineProps<{
  title: string
  description?: string
  icon?: string
  selected?: boolean
  disabled?: boolean
  disabledReason?: string
}>()

defineSlots<{ default?(): unknown; aside?(): unknown }>()
</script>

<template>
  <button
    type="button"
    class="grid min-h-21.5 cursor-pointer content-start gap-1.5 rounded-field border p-3.5 text-left transition-colors disabled:cursor-not-allowed disabled:opacity-45"
    :class="selected ? 'border-primary bg-primary/14 shadow-[inset_0_0_0_1px_var(--ui-primary)]' : 'border-default bg-(--aurora-glass) hover:bg-elevated'"
    :aria-pressed="selected"
    :disabled="disabled"
  >
    <span class="flex items-center gap-2 text-[14.5px] font-semibold text-highlighted">
      <UIcon
        v-if="icon"
        :name="icon"
        class="size-4.5 shrink-0 text-primary"
      />
      <span class="min-w-0 flex-1 truncate">{{ title }}</span>
      <slot name="aside" />
    </span>
    <span
      v-if="description"
      class="text-xs leading-snug text-muted"
    >{{ description }}</span>
    <span
      v-if="disabled && disabledReason"
      class="flex items-center gap-1.5 text-[11.5px] text-warning"
    >
      <UIcon
        name="i-lucide-triangle-alert"
        class="size-3.5 shrink-0"
      />
      {{ disabledReason }}
    </span>
    <slot />
  </button>
</template>
