<script setup lang="ts">
import SegmentedControl from '@renderer/components/ui/SegmentedControl.vue'
import { useIntiface } from '@renderer/composables/useIntiface'
import type { IntifacePatternDefinition } from '@renderer/composables/useIntifacePatterns'
import { toDefinition, useIntifacePatterns } from '@renderer/composables/useIntifacePatterns'
import { PATTERN_MAX_MS, PATTERN_SAMPLE_MS } from '@renderer/composables/useSavedPatterns'
import { useStopEverything } from '@renderer/composables/useStopEverything'
import type { IntifaceActuatorRef } from '@toolkitosc/shared-ui'
import { INTIFACE_PATTERN_OFF_ID, PatternRecorderBase, useIntifacePatternControl } from '@toolkitosc/shared-ui'
import { computed, onBeforeUnmount, ref, useTemplateRef, watch } from 'vue'

// Records a toy pattern from a slider (see PatternRecorderBase), with the picked toy following live.
// With `builtin` set, it only shows that pattern and plays it.
const props = defineProps<{
  builtin?: IntifacePatternDefinition
}>()

const samples = defineModel<number[]>('samples', { default: () => [] })

// Not a real control: the id "Play on <toy>" runs under, so Stop everything and other patterns on
// the same toy stop it like any other.
const PREVIEW_ID = 'pattern-preview'
// A built-in pattern has no length of its own; the graph shows this much of it.
const BUILTIN_WINDOW_MS = 8000

const { devices, setActuatorIntensity } = useIntiface()
const { playDefinition, playPattern, releaseActuators } = useIntifacePatterns()
const { controlValue: patternValues } = useIntifacePatternControl()
const { paused } = useStopEverything()

const recorder = useTemplateRef<InstanceType<typeof PatternRecorderBase>>('recorder')

// Toy follows the slider: one toy (all of its motors) or none.
const toy = ref<string>('none')

const toyItems = computed(() => [
  ...Array.from(devices.value.values()).map((device) => ({ value: String(device.index), label: device.name })),
  { value: 'none', label: 'No toy' }
])

// The first toy until you pick one yourself - toys often connect after the page opens.
let picked = false

watch(
  toyItems,
  (items) => {
    if (!picked || !items.some((item) => item.value === toy.value)) toy.value = items[0]!.value
  },
  { immediate: true }
)

const pickToy = (value: string): void => {
  picked = true
  toy.value = value
}

const actuators = computed<IntifaceActuatorRef[]>(() => {
  const device = toy.value === 'none' ? undefined : devices.value.get(Number(toy.value))

  return (
    device?.actuators.map((actuator) => ({
      deviceIndex: device.index,
      deviceName: device.name,
      actuatorIndex: actuator.index,
      actuatorDescription: actuator.description,
      actuatorType: actuator.actuatorType
    })) ?? []
  )
})

const toyName = computed(() => toyItems.value.find((item) => item.value === toy.value)?.label)

// What the graph and the looping slider show: the take, or a stretch of the built-in pattern.
const shown = computed<number[]>(() => {
  if (!props.builtin) return samples.value

  return Array.from({ length: BUILTIN_WINDOW_MS / PATTERN_SAMPLE_MS }, (_, i) => Math.round(props.builtin!.value(i * PATTERN_SAMPLE_MS) * 100))
})

const note = ref('')

const follow = (value: number): void => {
  if (!paused.value && actuators.value.length) setActuatorIntensity(actuators.value, value / 100)
}

const onStart = (): void => {
  note.value = ''
  stopPreview()
  if (actuators.value.length) releaseActuators(actuators.value)
}

const onFinish = (kept: boolean): void => {
  if (!kept) note.value = 'That was under 1 s. Hold the slider a little longer.'
}

// --- Play on <toy> ---

const previewing = computed(() => patternValues.value.get(PREVIEW_ID) === PREVIEW_ID)
// The toy the preview started on, which is the one to stop even after you pick another.
let previewActuators: IntifaceActuatorRef[] = []

const stopPreview = (): void => {
  if (previewing.value) playPattern(PREVIEW_ID, INTIFACE_PATTERN_OFF_ID, previewActuators)
}

const togglePreview = (): void => {
  if (previewing.value) return stopPreview()

  const pattern = props.builtin ?? toDefinition({ id: PREVIEW_ID, name: 'Preview', samples: samples.value })

  previewActuators = actuators.value
  playDefinition(PREVIEW_ID, { ...pattern, id: PREVIEW_ID }, previewActuators)
  // The playhead starts over with the toy, so what you see is what it plays.
  recorder.value?.restartLoop()
}

// A new take or another toy: the old preview no longer matches.
watch([samples, toy], () => stopPreview())

onBeforeUnmount(stopPreview)

const fmt = (ms: number): string => `${(ms / 1000).toFixed(1)} s`

const state = computed(() => recorder.value?.state ?? 'idle')

const status = computed(() => {
  if (props.builtin) return `Built-in · ${props.builtin.name}`
  if (state.value === 'recording') return `Recording · ${fmt(recorder.value?.takeMs ?? 0)} of ${fmt(PATTERN_MAX_MS)}`
  if (state.value === 'loop') return `Looping · ${fmt(recorder.value?.lengthMs ?? 0)}`

  return 'Ready to record'
})

const hint = computed(() => {
  if (props.builtin) return 'Built-in patterns can’t be changed. Make your own with New.'
  if (note.value) return note.value
  if (state.value === 'recording') return 'Let go of the slider to stop recording.'
  if (state.value === 'loop') return 'Happy with it? Save. If not, grab the slider again to record over it.'

  return 'The recording loops until the pattern is turned off.'
})
</script>

<template>
  <section class="flex min-h-0 flex-1 flex-col gap-3.5 rounded-2xl glass p-5">
    <div class="flex flex-wrap items-center gap-3.5">
      <div class="flex flex-1 items-center gap-2.5 text-lg font-semibold text-highlighted">
        <span
          v-if="state === 'recording'"
          class="size-2.5 animate-pulse rounded-full bg-error"
        />
        <span class="tabular-nums">{{ status }}</span>
      </div>
      <span class="text-sm text-muted">Toy follows the slider</span>
      <SegmentedControl
        :model-value="toy"
        accent
        aria-label="Toy follows the slider"
        :items="toyItems"
        @update:model-value="pickToy"
      />
    </div>

    <PatternRecorderBase
      ref="recorder"
      :samples="shown"
      :readonly="!!builtin"
      :idle-text="`Whatever you do is recorded. Let go to stop. Up to ${fmt(PATTERN_MAX_MS)}.`"
      @update:samples="samples = $event"
      @live="follow"
      @start="onStart"
      @finish="onFinish"
    />

    <div class="flex flex-wrap items-center gap-3">
      <p
        class="flex-1 text-sm"
        :class="note ? 'text-warning' : 'text-muted'"
      >
        {{ hint }}
      </p>
      <UButton
        v-if="state === 'loop'"
        :icon="previewing ? 'i-lucide-square' : 'i-lucide-play'"
        :color="previewing ? 'neutral' : 'primary'"
        :variant="previewing ? 'subtle' : 'solid'"
        :disabled="!actuators.length || paused"
        :title="paused ? 'Stop everything is on' : !actuators.length ? 'Pick a toy first' : undefined"
        @click="togglePreview"
      >
        {{ previewing ? `Playing on ${toyName} · Stop` : actuators.length ? `Play on ${toyName}` : 'Play on a toy' }}
      </UButton>
    </div>
  </section>
</template>
