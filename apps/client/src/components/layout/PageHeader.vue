<script setup lang="ts">
// Title block for PageLayout's #header slot: breadcrumb trail, title (truncated, full text on
// hover) and a muted meta line, with page actions on the right.
defineProps<{
  title: string
  meta?: string
  crumbs?: { label: string; to?: string }[]
}>()

defineSlots<{ actions?(): unknown }>()
</script>

<template>
  <div class="grid min-w-0 flex-1 gap-0.5">
    <nav
      v-if="crumbs?.length"
      aria-label="Breadcrumb"
      class="flex min-w-0 items-center gap-1.5 overflow-hidden text-[12.5px] whitespace-nowrap text-muted"
    >
      <template
        v-for="crumb in crumbs"
        :key="crumb.label"
      >
        <RouterLink
          v-if="crumb.to"
          :to="crumb.to"
          class="underline decoration-(--ui-border-accented) underline-offset-3 hover:text-default"
        >
          {{ crumb.label }}
        </RouterLink>
        <span v-else>{{ crumb.label }}</span>
        <span
          v-if="crumb.to"
          aria-hidden="true"
        >/</span>
      </template>
    </nav>

    <h1
      class="truncate text-[28px] leading-tight font-semibold text-highlighted"
      :title="title"
    >
      {{ title }}
    </h1>

    <p
      v-if="meta"
      class="text-[13px] text-muted"
    >
      {{ meta }}
    </p>
  </div>

  <div
    v-if="$slots.actions"
    class="flex shrink-0 flex-wrap items-center gap-2"
  >
    <slot name="actions" />
  </div>
</template>
