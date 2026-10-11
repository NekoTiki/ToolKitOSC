<script setup lang="ts">
import SegmentedControl from '@renderer/components/ui/SegmentedControl.vue'
import { useIntiface } from '@renderer/composables/useIntiface'
import type { IntifacePatternDefinition } from '@renderer/composables/useIntifacePatterns'
import { toDefinition, useIntifacePatterns } from '@renderer/composables/useIntifacePatterns'
import { PATTERN_MAX_MS, PATTERN_MIN_MS, PATTERN_SAMPLE_MS } from '@renderer/composables/useSavedPatterns'
import { useStopEverything } from '@renderer/composables/useStopEverything'
import type { IntifaceActuatorRef } from '@toolkitosc/shared-ui'
import { INTIFACE_PATTERN_OFF_ID, useIntifacePatternControl } from '@toolkitosc/shared-ui'
import { computed, onBeforeUnmount, ref, shallowRef, watch } from 'vue'

// Records a toy pattern from a slider: grab the slider and whatever you do is recorded, one value
// every PATTERN_SAMPLE_MS, with the picked toy following live. Letting go loops the take. Grabbing it
// again records over it. With `builtin` set, it only shows that pattern and plays it.
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

// --- Recording ---

const recording = ref(false)
// The take being recorded; the model only changes once it's long enough to keep.
const take = shallowRef<number[]>([])
const current = ref(0)
const note = ref('')
let recordTimer: ReturnType<typeof setInterval> | undefined

const faderRef = ref<HTMLElement>()
// Knob height and the track's inner padding, in px - see the template.
const KNOB = 44
const PAD = 6

const valueAt = (event: PointerEvent): number => {
  const rect = faderRef.value!.getBoundingClientRect()
  const travel = Math.max(rect.height - 2 * PAD - KNOB, 1)

  return Math.round(Math.max(0, Math.min(100, ((rect.bottom - PAD - KNOB / 2 - event.clientY) / travel) * 100)))
}

const follow = (value: number): void => {
  if (!paused.value && actuators.value.length) setActuatorIntensity(actuators.value, value / 100)
}

const finish = (): void => {
  if (!recording.value) return

  clearInterval(recordTimer)
  recording.value = false
  follow(0)

  if (take.value.length * PATTERN_SAMPLE_MS < PATTERN_MIN_MS) {
    note.value = 'That was under 1 s. Hold the slider a little longer.'
  } else {
    samples.value = take.value
  }

  take.value = []
  restartLoop()
}

const onPointerDown = (event: PointerEvent): void => {
  // Left button (or a VR trigger) only - right-click is reserved for context menus.
  if (event.button !== 0 || props.builtin || !faderRef.value) return

  event.preventDefault()
  faderRef.value.setPointerCapture(event.pointerId)
  stopPreview()
  if (actuators.value.length) releaseActuators(actuators.value)

  note.value = ''
  current.value = valueAt(event)
  take.value = [current.value]
  recording.value = true
  follow(current.value)

  recordTimer = setInterval(() => {
    take.value = [...take.value, current.value]
    follow(current.value)

    if (take.value.length * PATTERN_SAMPLE_MS >= PATTERN_MAX_MS) finish()
  }, PATTERN_SAMPLE_MS)
}

const onPointerMove = (event: PointerEvent): void => {
  if (recording.value) current.value = valueAt(event)
}

// --- Looping ---

// Drives the playhead and the slider replaying the take. The toy only plays it on "Play on <toy>".
const loopStart = ref(Date.now())
const now = ref(Date.now())
let frame: number | undefined

const tickLoop = (): void => {
  now.value = Date.now()
  frame = requestAnimationFrame(tickLoop)
}

const restartLoop = (): void => {
  loopStart.value = Date.now()
  now.value = loopStart.value
}

frame = requestAnimationFrame(tickLoop)

const lengthMs = computed(() => shown.value.length * PATTERN_SAMPLE_MS)
const elapsed = computed(() => (lengthMs.value ? (now.value - loopStart.value) % lengthMs.value : 0))

const state = computed<'idle' | 'recording' | 'loop'>(() => (recording.value ? 'recording' : shown.value.length ? 'loop' : 'idle'))

const sliderValue = computed(() => {
  if (state.value === 'recording') return current.value
  if (state.value === 'loop') return shown.value[Math.floor(elapsed.value / PATTERN_SAMPLE_MS) % shown.value.length] ?? 0

  return 0
})

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

  // The playhead starts over with the toy, so what you see is what it plays.
  previewActuators = actuators.value
  playDefinition(PREVIEW_ID, { ...pattern, id: PREVIEW_ID }, previewActuators)
  restartLoop()
}

// A new take or another toy: the old preview no longer matches.
watch([samples, toy], () => stopPreview())

onBeforeUnmount(() => {
  if (frame !== undefined) cancelAnimationFrame(frame)
  if (recording.value) {
    clearInterval(recordTimer)
    follow(0)
  }
  stopPreview()
})

// --- Drawing ---

const fmt = (ms: number): string => `${(ms / 1000).toFixed(1)} s`

const drawn = computed(() => (state.value === 'recording' ? take.value : shown.value))

// While recording, the graph spans at least 10 s and grows with the take, so the line has room.
const spanMs = computed(() =>
  state.value === 'recording' ? Math.max(10_000, drawn.value.length * PATTERN_SAMPLE_MS) : Math.max((drawn.value.length - 1) * PATTERN_SAMPLE_MS, 1)
)

const xOf = (index: number): number => ((index * PATTERN_SAMPLE_MS) / spanMs.value) * 1000

const line = computed(() => drawn.value.map((value, index) => `${xOf(index).toFixed(1)},${100 - value}`).join(' '))
const area = computed(() => (drawn.value.length ? `0,100 ${line.value} ${xOf(drawn.value.length - 1).toFixed(1)},100` : ''))

const penX = computed(() => xOf(Math.max(drawn.value.length - 1, 0)) / 1000)
const headX = computed(() => (lengthMs.value ? elapsed.value / lengthMs.value : 0))

const status = computed(() => {
  if (props.builtin) return `Built-in · ${props.builtin.name}`
  if (state.value === 'recording') return `Recording · ${fmt(take.value.length * PATTERN_SAMPLE_MS)} of ${fmt(PATTERN_MAX_MS)}`
  if (state.value === 'loop') return `Looping · ${fmt(lengthMs.value)}`

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

    <div class="flex min-h-72 flex-1 gap-3.5">
      <div class="relative min-w-0 flex-1 overflow-hidden rounded-xl well">
        <div class="pointer-events-none absolute inset-x-0 inset-y-5 flex flex-col justify-between">
          <div
            v-for="i in 4"
            :key="i"
            class="border-t border-dashed border-white/9"
          />
          <div class="border-t border-white/16" />
        </div>

        <div
          v-if="state === 'idle'"
          class="absolute inset-0 grid place-items-center px-10 text-center text-[15px] text-muted"
        >
          <div>
            <b class="mb-1 block text-xl font-semibold text-highlighted">Grab the slider and move it</b>
            Whatever you do is recorded. Let go to stop. Up to {{ fmt(PATTERN_MAX_MS) }}.
          </div>
        </div>

        <svg
          viewBox="0 0 1000 100"
          preserveAspectRatio="none"
          aria-hidden="true"
          class="absolute inset-x-4 inset-y-5 h-[calc(100%-2.5rem)] w-[calc(100%-2rem)] overflow-visible"
        >
          <polygon
            :points="area"
            class="fill-secondary/14"
          />
          <polyline
            :points="line"
            class="fill-none stroke-secondary"
            stroke-width="3"
            stroke-linejoin="round"
            vector-effect="non-scaling-stroke"
          />
        </svg>

        <div
          v-if="state === 'recording'"
          class="pointer-events-none absolute -mt-2.25 -ml-2.25 size-4.5 rounded-full border-3 border-secondary bg-white shadow-[0_0_16px_var(--ui-secondary)]"
          :style="{ left: `calc(1rem + (100% - 2rem) * ${penX})`, top: `calc(1.25rem + (100% - 2.5rem) * ${(100 - current) / 100})` }"
        />
        <div
          v-else-if="state === 'loop'"
          class="pointer-events-none absolute inset-y-3 w-0.5 bg-primary shadow-[0_0_12px_var(--ui-primary)]"
          :style="{ left: `calc(1rem + (100% - 2rem) * ${headX})` }"
        />

        <div class="absolute inset-x-4 bottom-1 flex justify-between text-[11.5px] text-dimmed tabular-nums">
          <span>0 s</span>
          <span>{{ fmt(state === 'recording' ? spanMs : lengthMs) }}</span>
        </div>
      </div>

      <!-- A tall linear fader with a big knob, so a mouse, a finger or a VR pointer (hold the trigger)
      can all drive it. Pointer capture keeps the take going when the pointer slips off. -->
      <div
        ref="faderRef"
        role="slider"
        aria-orientation="vertical"
        aria-label="Recording slider. Hold and move it to record."
        aria-valuemin="0"
        aria-valuemax="100"
        :aria-valuenow="sliderValue"
        :aria-disabled="!!builtin"
        class="relative w-28 shrink-0 touch-none overflow-hidden rounded-xl border well"
        :class="[
          recording ? 'cursor-grabbing border-error/70' : 'border-default',
          builtin ? 'opacity-60' : recording ? '' : 'cursor-grab'
        ]"
        @pointerdown="onPointerDown"
        @pointermove="onPointerMove"
        @pointerup="finish"
        @pointercancel="finish"
      >
        <div
          class="pointer-events-none absolute inset-x-0 bottom-0 bg-linear-to-t from-secondary/45 to-secondary/18"
          :style="{ height: `calc(${PAD + KNOB / 2}px + (100% - ${2 * PAD + KNOB}px) * ${sliderValue / 100})` }"
        />
        <div
          class="pointer-events-none absolute inset-x-2.5 grid place-items-center rounded-lg border-3 border-secondary bg-white font-bold text-black/80 tabular-nums shadow-[0_4px_14px_rgb(0_0_0/0.45)]"
          :style="{ height: `${KNOB}px`, bottom: `calc(${PAD}px + (100% - ${2 * PAD + KNOB}px) * ${sliderValue / 100})` }"
        >
          {{ sliderValue }}%
        </div>
      </div>
    </div>

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
