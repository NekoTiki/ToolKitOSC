<script setup lang="ts">
// A viewer's round avatar on the share page: their Discord picture, or for a guest their initials
// on a colour picked from their id, so the same guest keeps the same colour everywhere.
const props = defineProps<{
  id: string
  name: string
  avatar?: string
  size?: 'sm' | 'md'
}>()

const COLORS = ['#8b5cf6', '#ec4899', '#0ea5e9', '#f59e0b', '#10b981', '#ef4444', '#6366f1', '#14b8a6']

const color = computed(() => {
  let h = 0

  for (const char of props.id) h = (h * 31 + char.charCodeAt(0)) >>> 0

  return COLORS[h % COLORS.length]
})

const initials = computed(() =>
  props.name
    .split(/\s+/)
    .filter(Boolean)
    .slice(0, 2)
    .map((word) => word[0]!.toUpperCase())
    .join('')
)

const failed = ref(false)
</script>

<template>
  <img
    v-if="avatar && !failed"
    :src="avatar"
    alt=""
    class="shrink-0 rounded-full object-cover"
    :class="size === 'md' ? 'size-8' : 'size-7'"
    @error="failed = true"
  >
  <span
    v-else
    aria-hidden="true"
    class="grid shrink-0 place-items-center rounded-full font-bold text-white"
    :class="size === 'md' ? 'size-8 text-xs' : 'size-7 text-[11px]'"
    :style="{ background: color }"
  >{{ initials }}</span>
</template>
