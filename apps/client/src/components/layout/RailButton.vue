<script setup lang="ts">
// One AppRail entry, 52px tall for a VR pointer. Expanded, the page's name sits next to the icon;
// collapsed, it's a 52px icon-only square with the label as the tooltip. The label is
// always the accessible name.
//
// `busy` spins a ring around the icon while the page's work runs in the background; `alert` is a
// dot saying how that work ended, shown until the user opens the page. `addon` gives a page an
// add-on brings a dashed outline, to tell it from the core pages.
defineProps<{
  icon: string
  label: string
  active?: boolean
  disabled?: boolean
  badge?: number
  busy?: boolean
  alert?: 'success' | 'error' | null
  addon?: boolean
  collapsed?: boolean
}>()
</script>

<template>
  <button
    type="button"
    class="relative flex h-13 w-full shrink-0 cursor-pointer items-center rounded-2xl transition-colors disabled:cursor-not-allowed disabled:opacity-40"
    :class="[
      // The 9px padding puts the icon where it sits when collapsed, so it doesn't jump on toggle.
      collapsed ? 'justify-center' : 'gap-2.5 px-2.25',
      active ? 'bg-primary/15 text-highlighted' : 'text-muted hover:bg-elevated/60 hover:text-default',
      addon && 'border border-dashed border-primary/45'
    ]"
    :disabled="disabled"
    :aria-current="active ? 'page' : undefined"
    :aria-busy="busy || undefined"
    :title="collapsed ? (busy ? `${label} - working…` : label) : undefined"
    :aria-label="label"
  >
    <span
      class="relative grid size-8.5 shrink-0 place-items-center"
    >
      <span
        v-if="busy"
        class="rail-busy-ring absolute inset-0 rounded-full"
        aria-hidden="true"
      />
      <UIcon
        :name="icon"
        class="size-5.5"
        :class="[{ 'text-primary': active || busy }, busy && 'rail-busy-icon']"
      />
    </span>
    <span
      v-if="!collapsed"
      class="text-sm whitespace-nowrap"
    >{{ label }}</span>
    <span
      v-if="badge"
      class="grid h-4.5 min-w-4.5 place-items-center rounded-full bg-primary px-1 font-mono text-[10px] font-semibold text-inverted"
      :class="collapsed ? 'absolute top-1 right-1' : 'ml-auto'"
    >{{ badge }}</span>
    <span
      v-else-if="alert"
      class="size-3 shrink-0 rounded-full ring-2 ring-(--ui-bg)"
      :class="[collapsed ? 'absolute top-1.5 right-1.5' : 'ml-auto', alert === 'error' ? 'bg-error' : 'rail-alert-pulse bg-success']"
      :aria-label="alert === 'error' ? 'Failed' : 'Ready'"
    />
  </button>
</template>

<style scoped>
/* A primary→secondary arc chasing around the icon, cut to a 2px ring by the mask. */
.rail-busy-ring {
  background: conic-gradient(from 0deg, transparent 0 30%, var(--ui-secondary) 60%, var(--ui-primary) 100%);
  mask: radial-gradient(farthest-side, transparent calc(100% - 2.5px), #000 calc(100% - 2px));
  animation: rail-spin 1.1s linear infinite;
}

.rail-busy-icon {
  animation: rail-breathe 1.6s ease-in-out infinite;
}

.rail-alert-pulse {
  animation: rail-ping 1.8s ease-out infinite;
}

@keyframes rail-spin {
  to {
    transform: rotate(1turn);
  }
}

@keyframes rail-breathe {
  50% {
    opacity: 0.55;
    transform: scale(0.88);
  }
}

@keyframes rail-ping {
  0% {
    box-shadow: 0 0 0 0 color-mix(in oklab, var(--ui-success) 70%, transparent);
  }
  100% {
    box-shadow: 0 0 0 8px transparent;
  }
}

@media (prefers-reduced-motion: reduce) {
  .rail-busy-ring {
    animation-duration: 3s;
  }

  .rail-busy-icon,
  .rail-alert-pulse {
    animation: none;
  }
}
</style>
