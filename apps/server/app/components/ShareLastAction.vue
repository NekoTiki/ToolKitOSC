<script setup lang="ts">
import type { ControlGroup, ViewerActionMessage } from '@toolkitosc/shared-ui'
import { describeCommand } from '@toolkitosc/shared-ui'

// The share page's last-action bar: who last used a control, what they did, and how long ago.
// Only commands the host actually ran get here (see host.ts' command-accepted).
const props = defineProps<{
  action: ViewerActionMessage
  controlGroups: ControlGroup[]
  you: string | null
}>()

const now = useNow({ interval: 1000 })

const control = computed(() =>
  props.controlGroups.find((group) => group.id === props.action.groupId)?.controls.find((c) => c.id === props.action.controlId)
)

const who = computed(() => {
  const viewer = props.action.viewer

  if (!viewer) return 'Someone'

  return viewer.id === props.you ? 'You' : viewer.name
})

const what = computed(() => describeCommand(control.value, props.action.type, props.action.value))

const ago = computed(() => {
  const seconds = Math.max(0, Math.floor((now.value.getTime() - props.action.at) / 1000))

  if (seconds < 5) return 'now'
  if (seconds < 60) return `${seconds}s ago`
  if (seconds < 3600) return `${Math.floor(seconds / 60)}m ago`

  return `${Math.floor(seconds / 3600)}h ago`
})
</script>

<template>
  <div
    class="flex h-10 min-w-0 items-center gap-3 overflow-hidden glass px-3.5 font-mono text-[11px] whitespace-nowrap text-muted"
    role="status"
    aria-live="polite"
  >
    <ViewerAvatar
      v-if="action.viewer"
      :id="action.viewer.id"
      :name="action.viewer.name"
      :avatar="action.viewer.avatar"
      size="sm"
    />
    <UIcon
      v-else
      name="i-lucide-activity"
      class="size-4 shrink-0 text-primary"
    />
    <span class="min-w-0 truncate text-highlighted">{{ who }} · {{ control?.name ?? action.controlName }}</span>
    <span class="min-w-0 truncate text-primary">{{ what }}</span>
    <span class="ml-auto shrink-0">{{ ago }}</span>
  </div>
</template>
