<script lang="ts">
import type { IntifaceActuatorRef } from '@toolkitosc/shared-ui'
import { ref } from 'vue'

// Not a real control, like the Patterns page's preview: Stop everything and other patterns on the
// same toy stop it like any other. Shared by every row (rows come and go as the list scrolls), so
// one Try it plays at a time and only its row shows Stop.
const PREVIEW_ID = 'activity-preview'
const previewing = ref<{ commandId?: number; actuators: IntifaceActuatorRef[] } | null>(null)

// Stops the Try it preview, if any. The Activity page calls it when you leave.
export const stopViewerPatternPreview = (): void => {
  if (!previewing.value) return

  useIntifacePatterns().playPattern(PREVIEW_ID, INTIFACE_PATTERN_OFF_ID, previewing.value.actuators)
  previewing.value = null
}
</script>

<script setup lang="ts">
import PatternSparkline from '@renderer/components/patterns/PatternSparkline.vue'
import { useControls } from '@renderer/composables/useControls'
import { toDefinition, useIntifacePatterns } from '@renderer/composables/useIntifacePatterns'
import { PATTERN_SAMPLE_MS } from '@renderer/composables/useSavedPatterns'
import { useStopEverything } from '@renderer/composables/useStopEverything'
import type { Command } from '@renderer/db/commands.db'
import { INTIFACE_PATTERN_OFF_ID, useIntifacePatternControl } from '@toolkitosc/shared-ui'
import { computed } from 'vue'
import { useRouter } from 'vue-router'

// A viewer's own pattern in Activity (and a viewer's history): its line, how long it is and ran,
// and Try it / Save to my patterns, so a good one can become one of yours.
const props = defineProps<{
  command: Command & { samples: number[] }
  viewerName?: string
}>()

const router = useRouter()
const { getControl } = useControls()
const { playDefinition } = useIntifacePatterns()
const { controlValue: patternValues } = useIntifacePatternControl()
const { paused } = useStopEverything()

const pattern = computed(() => toDefinition({ id: PREVIEW_ID, name: 'Viewer pattern', samples: props.command.samples }))

const fmt = (ms: number): string => (ms < 60_000 ? `${Math.round(ms / 1000)} s` : `${Math.round(ms / 60_000)} min`)

const details = computed(() => {
  const { samples, createdAt, endedAt } = props.command
  const parts = [`${((samples.length * PATTERN_SAMPLE_MS) / 1000).toFixed(1)} s per loop`, `up to ${Math.max(...samples)}%`]

  if (endedAt) parts.push(`ran ${fmt(endedAt - createdAt)}`)

  return parts.join(' · ')
})

const control = computed(() => {
  const found = getControl(props.command.groupId, props.command.controlId)

  return found?.type === 'intiface-pattern' ? found : undefined
})

const playing = computed(() => patternValues.value.get(PREVIEW_ID) === PREVIEW_ID && previewing.value?.commandId === props.command.id)

const tryIt = (): void => {
  if (!control.value) return
  if (playing.value) return stopViewerPatternPreview()

  stopViewerPatternPreview()
  previewing.value = { commandId: props.command.id, actuators: control.value.actuators }
  playDefinition(PREVIEW_ID, pattern.value, control.value.actuators)
}

const tryTitle = computed(() => {
  if (!control.value) return 'The control was deleted'
  if (paused.value) return 'Stop everything is on'

  return playing.value ? 'Stop' : `Try it on ${control.value.name}`
})

const save = (): void =>
  void router.push({ path: '/patterns/new', query: { fromCommand: String(props.command.id), ...(props.viewerName ? { by: props.viewerName } : {}) } })
</script>

<template>
  <div class="flex min-w-0 items-center gap-2">
    <span class="grid h-8.5 w-18 shrink-0 place-items-center rounded-xl bg-(--aurora-well)">
      <PatternSparkline
        :pattern="pattern"
        mine
      />
    </span>
    <small class="min-w-0 flex-1 truncate text-xs text-muted">{{ details }}</small>
    <UTooltip :text="tryTitle">
      <UButton
        :icon="playing ? 'i-lucide-square' : 'i-lucide-play'"
        color="neutral"
        variant="subtle"
        size="md"
        :aria-label="tryTitle"
        :disabled="!control || paused"
        @click="tryIt"
      />
    </UTooltip>
    <UTooltip text="Save to my patterns">
      <UButton
        icon="i-lucide-save"
        size="md"
        aria-label="Save to my patterns"
        @click="save"
      />
    </UTooltip>
  </div>
</template>
