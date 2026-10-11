<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from 'vue'

import { useIntifacePatternControl } from '../composables/useIntifacePatternControl'
import type { IntifacePatternRef } from '../types/controls'
import { INTIFACE_PATTERN_OFF_ID, VIEWER_PATTERN_ID } from '../types/controls'
import ControlBase from './ControlBase.vue'
import ControlOptions from './ControlOptions.vue'

// Same chip-picker treatment as ControlEnum.vue, for the same reason (every choice tappable and
// visible at once, no dropdown) - the only real difference is the value being played back over
// time client-side (see useIntifacePatterns.ts) instead of a single static value, which this
// widget doesn't need to know or care about; it just reflects whichever pattern id is currently
// active and emits whichever one gets tapped.
const props = defineProps<{
  controlId: string
  title: string
  icon?: string
  patterns: IntifacePatternRef[]
  // Pattern ids the active profile blocks for viewers. Never 'off'.
  disabledOptions?: (string | number)[]
  // Share page, when the control lets viewers record their own pattern: 'allowed' shows a Make one
  // chip, 'sign-in' shows it disabled (signed-in viewers only). Absent: no chip.
  patternMaker?: 'allowed' | 'sign-in'
  // The viewer pattern playing is this viewer's own.
  mine?: boolean
}>()

const emit = defineEmits<{
  (e: 'update:value', patternId: string): void
  (e: 'make'): void
}>()

const { controlValue, viewerPatterns } = useIntifacePatternControl()

const viewerPattern = computed(() => (controlValue.value.get(props.controlId) === VIEWER_PATTERN_ID ? (viewerPatterns.value.get(props.controlId) ?? {}) : null))

// Ticks once a second while a viewer pattern with an end time plays, for its countdown.
const now = ref(Date.now())
let clock: ReturnType<typeof setInterval> | undefined

watch(
  () => viewerPattern.value?.endsAt,
  (endsAt) => {
    clearInterval(clock)
    now.value = Date.now()
    if (endsAt) clock = setInterval(() => (now.value = Date.now()), 1000)
  },
  { immediate: true }
)

onBeforeUnmount(() => clearInterval(clock))

const viewerPatternLabel = computed(() => {
  if (!viewerPattern.value) return ''

  const { by, endsAt } = viewerPattern.value
  const name = props.mine ? 'Your pattern' : by ? `${by}'s pattern` : "A viewer's pattern"

  if (!endsAt) return name

  const seconds = Math.max(0, Math.round((endsAt - now.value) / 1000))

  return `${name} · ${Math.floor(seconds / 60)}:${String(seconds % 60).padStart(2, '0')}`
})

const currentPatternId = computed(() => controlValue.value.get(props.controlId) ?? INTIFACE_PATTERN_OFF_ID)

// 'Off' isn't one of `patterns` (a control's configurable allowedPatterns) - it's always offered,
// first, regardless of what the control's author picked.
const chips = computed<IntifacePatternRef[]>(() => [
  { id: INTIFACE_PATTERN_OFF_ID, name: 'Off', icon: 'i-lucide-power-off' },
  ...props.patterns
])

const MAKE_KEY = 'make'

const options = computed(() => [
  ...chips.value.map((chip) => ({
    key: chip.id,
    name: chip.name,
    icon: chip.icon,
    disabled: chip.id !== INTIFACE_PATTERN_OFF_ID && !!props.disabledOptions?.includes(chip.id),
    selected: chip.id === currentPatternId.value
  })),
  ...(viewerPattern.value ? [{ key: VIEWER_PATTERN_ID, name: viewerPatternLabel.value, icon: 'i-lucide-audio-waveform', selected: true }] : []),
  ...(props.patternMaker
    ? [
        {
          key: MAKE_KEY,
          name: 'Make one',
          icon: 'i-lucide-plus',
          selected: false,
          action: true,
          disabled: props.patternMaker === 'sign-in',
          disabledReason: 'sign in with Discord to make one'
        }
      ]
    : [])
])

const select = (index: number): void => {
  const key = options.value[index]?.key

  if (key === MAKE_KEY) emit('make')
  else if (key !== undefined && key !== VIEWER_PATTERN_ID) emit('update:value', String(key))
}
</script>

<template>
  <control-base
    :title="title"
    :icon="icon"
    kind="value"
    fill
  >
    <template #badge>
      <span
        class="grid size-5 shrink-0 place-items-center rounded-full bg-(--aurora-well) text-muted"
        title="Intiface"
      >
        <UIcon
          name="mdi:vibrate"
          class="size-3"
        />
      </span>
    </template>
    <ControlOptions
      :options="options"
      pill
      @select="select"
    />
  </control-base>
</template>
