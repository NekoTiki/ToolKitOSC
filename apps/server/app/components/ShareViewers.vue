<script setup lang="ts">
import type { PresenceEntry } from '@toolkitosc/shared-ui'

// "Here now": everyone on the share page, you first, then whoever did something most recently.
const props = defineProps<{
  viewers: PresenceEntry[]
  you: string | null
}>()

const now = useNow({ interval: 15_000 })

const sorted = computed(() =>
  [...props.viewers].sort((a, b) => {
    if (a.id === props.you) return -1
    if (b.id === props.you) return 1

    return (b.lastActiveAt ?? 0) - (a.lastActiveAt ?? 0)
  })
)

const activity = (viewer: PresenceEntry): string => {
  if (!viewer.lastActiveAt) return 'watching'

  const minutes = Math.floor((now.value.getTime() - viewer.lastActiveAt) / 60_000)

  if (minutes < 1) return 'active'
  if (minutes < 60) return `idle ${minutes}m`

  return `idle ${Math.floor(minutes / 60)}h`
}
</script>

<template>
  <div class="grid gap-1">
    <div class="flex justify-between px-2.5 pt-2 pb-0.5 font-mono text-[10.5px] tracking-widest text-muted uppercase">
      <span>Here now</span>
      <span>{{ viewers.length }}</span>
    </div>
    <ul class="grid max-h-55 gap-1 overflow-y-auto">
      <li
        v-for="viewer in sorted"
        :key="viewer.id"
        class="flex h-10 min-w-0 items-center gap-2.5 rounded-field px-2 text-[13.5px]"
      >
        <ViewerAvatar
          :id="viewer.id"
          :name="viewer.name"
          :avatar="viewer.avatar"
        />
        <span
          class="min-w-0 flex-1 truncate"
          :title="viewer.name"
        >{{ viewer.name }}</span>
        <span
          v-if="viewer.id === you"
          class="shrink-0 rounded-full well px-1.5 py-0.5 font-mono text-[10px] text-muted"
        >you</span>
        <span
          v-else
          class="shrink-0 font-mono text-[10px]"
          :class="activity(viewer) === 'active' ? 'text-primary' : 'text-muted'"
        >{{ activity(viewer) }}</span>
      </li>
    </ul>
  </div>
</template>
