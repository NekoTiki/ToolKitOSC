<script setup lang="ts">
import { api } from '@renderer/lib/tauri-bridge'

// One release's notes, already rendered by useChangelog. markdown-it runs with html: false, so the
// body can't carry its own HTML - v-html only ever sees markdown-it's output.
defineProps<{ html: string }>()

// Links open in the system browser instead of navigating the app's webview.
function onClick(event: MouseEvent): void {
  const link = (event.target as HTMLElement).closest<HTMLAnchorElement>('a[data-external]')
  if (!link) return
  event.preventDefault()
  api.openUrl(link.href)
}
</script>

<!-- eslint-disable vue/no-v-html, tailwindcss/no-custom-classname -->
<template>
  <div
    class="release-notes"
    @click="onClick"
    v-html="html"
  />
</template>

<style scoped>
.release-notes {
  display: grid;
  gap: 0.6rem;
  font-size: 14px;
  line-height: 1.55;
  color: var(--ui-text);
}

.release-notes :deep(h1),
.release-notes :deep(h2),
.release-notes :deep(h3) {
  margin-top: 0.4rem;
  font-size: 15px;
  font-weight: 600;
  color: var(--ui-text-highlighted);
}

.release-notes :deep(ul),
.release-notes :deep(ol) {
  display: grid;
  gap: 0.35rem;
  padding-left: 1.1rem;
  list-style: disc;
}

.release-notes :deep(ol) {
  list-style: decimal;
}

.release-notes :deep(li > ul) {
  margin-top: 0.35rem;
  list-style: circle;
}

.release-notes :deep(strong) {
  font-weight: 600;
  color: var(--ui-text-highlighted);
}

.release-notes :deep(code) {
  padding: 0.05rem 0.35rem;
  border-radius: 0.375rem;
  background: var(--aurora-well);
  font-size: 0.9em;
}

.release-notes :deep(a) {
  color: var(--ui-primary);
  text-decoration: underline;
  text-underline-offset: 2px;
  overflow-wrap: anywhere;
}
</style>
