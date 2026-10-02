<script setup lang="ts">
// The frame every page renders into, to the right of AppRail: an optional context column (a
// page's own list - groups, presets, viewers, settings sections...) and the page itself, with a
// fixed header, a scrolling body and an optional sticky footer (Save/Cancel bars).
defineSlots<{
  sidebar?(): unknown
  header?(): unknown
  default(): unknown
  // A non-scrolling strip under the body (e.g. the Controls page's live command line).
  bar?(): unknown
  footer?(): unknown
}>()
</script>

<template>
  <div
    class="grid min-h-0 min-w-0"
    :class="$slots.sidebar ? 'grid-cols-[16.5rem_minmax(0,1fr)] max-lg:grid-cols-[13.5rem_minmax(0,1fr)]' : 'grid-cols-1'"
  >
    <aside
      v-if="$slots.sidebar"
      class="flex min-h-0 min-w-0 flex-col gap-2.5 border-r border-default p-3.5"
    >
      <slot name="sidebar" />
    </aside>

    <section class="relative flex min-h-0 min-w-0 flex-col">
      <header
        v-if="$slots.header"
        class="flex min-w-0 shrink-0 flex-wrap items-center gap-3 px-5.5 pt-3.5 pb-3"
      >
        <slot name="header" />
      </header>

      <div class="min-h-0 flex-1 overflow-auto px-5.5 pb-5.5">
        <slot />
      </div>

      <slot name="bar" />

      <footer
        v-if="$slots.footer"
        class="flex shrink-0 items-center justify-end gap-2.5 border-t border-default bg-default/75 px-5.5 py-3"
      >
        <slot name="footer" />
      </footer>
    </section>
  </div>
</template>
