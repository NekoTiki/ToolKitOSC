<script setup lang="ts">
import { computed, onBeforeUnmount, ref, shallowRef } from 'vue'

import { PATTERN_MAX_MS, PATTERN_MIN_MS, PATTERN_SAMPLE_MS } from '../types/controls'

// The graph and the tall slider a toy pattern is recorded with, shared by the app's Patterns page
// and the share page. Grab the slider and whatever you do is recorded, one value (0-100) every
// PATTERN_SAMPLE_MS; letting go keeps the take, which then loops: a playhead runs over the line and
// the slider replays it. Grabbing it again records over it. What the toy does is up to the parent
// (see the `live` event).
const props = withDefaults(
  defineProps<{
    // Longest take, in ms. Recording stops by itself there.
    maxMs?: number
    // The top of the slider, 0-100. Above it the track is hatched, the knob stops there and nothing
    // higher is recorded.
    maxValue?: number
    // Shown in the hatched band, e.g. "Above NekoTiki's limit (70%)".
    limitLabel?: string
    // Show and replay `samples` only; the slider can't be grabbed.
    readonly?: boolean
    idleTitle?: string
    idleText?: string
  }>(),
  {
    maxMs: PATTERN_MAX_MS,
    maxValue: 100,
    limitLabel: undefined,
    readonly: false,
    idleTitle: 'Grab the slider and move it',
    idleText: 'Whatever you do is recorded. Let go to stop.'
  }
)

const samples = defineModel<number[]>('samples', { default: () => [] })

const emit = defineEmits<{
  // Every PATTERN_SAMPLE_MS while recording (0-100), and 0 when it stops.
  (e: 'live', value: number): void
  (e: 'start'): void
  // Recording stopped. `kept`: the take was long enough and is now `samples`.
  (e: 'finish', kept: boolean): void
}>()

const recording = ref(false)
// The take being recorded; the model only changes once it's long enough to keep.
const take = shallowRef<number[]>([])
const current = ref(0)
let recordTimer: ReturnType<typeof setInterval> | undefined

const top = computed(() => Math.max(0, Math.min(100, props.maxValue)))

const faderRef = ref<HTMLElement>()
// Knob height and the track's inner padding, in px - see the template.
const KNOB = 44
const PAD = 6

const valueAt = (event: PointerEvent): number => {
  const rect = faderRef.value!.getBoundingClientRect()
  const travel = Math.max(rect.height - 2 * PAD - KNOB, 1)
  const value = ((rect.bottom - PAD - KNOB / 2 - event.clientY) / travel) * 100

  return Math.round(Math.max(0, Math.min(top.value, value)))
}

const finish = (): void => {
  if (!recording.value) return

  clearInterval(recordTimer)
  recording.value = false
  emit('live', 0)

  const kept = take.value.length * PATTERN_SAMPLE_MS >= PATTERN_MIN_MS

  if (kept) samples.value = take.value

  take.value = []
  restartLoop()
  emit('finish', kept)
}

const onPointerDown = (event: PointerEvent): void => {
  // Left button (or a VR trigger) only - right-click is reserved for context menus.
  if (event.button !== 0 || props.readonly || !faderRef.value) return

  event.preventDefault()
  faderRef.value.setPointerCapture(event.pointerId)

  current.value = valueAt(event)
  take.value = [current.value]
  recording.value = true
  emit('start')
  emit('live', current.value)

  recordTimer = setInterval(() => {
    take.value = [...take.value, current.value]
    emit('live', current.value)

    if (take.value.length * PATTERN_SAMPLE_MS >= Math.min(props.maxMs, PATTERN_MAX_MS)) finish()
  }, PATTERN_SAMPLE_MS)
}

const onPointerMove = (event: PointerEvent): void => {
  if (recording.value) current.value = valueAt(event)
}

// Drives the playhead and the slider replaying the take.
const loopStart = ref(Date.now())
const now = ref(Date.now())
let frame: number | undefined

const tickLoop = (): void => {
  now.value = Date.now()
  frame = requestAnimationFrame(tickLoop)
}

// Starts the loop over, e.g. so it lines up with a toy that just started playing it.
const restartLoop = (): void => {
  loopStart.value = Date.now()
  now.value = loopStart.value
}

frame = requestAnimationFrame(tickLoop)

onBeforeUnmount(() => {
  if (frame !== undefined) cancelAnimationFrame(frame)
  if (recording.value) {
    clearInterval(recordTimer)
    emit('live', 0)
  }
})

const lengthMs = computed(() => samples.value.length * PATTERN_SAMPLE_MS)
const elapsed = computed(() => (lengthMs.value ? (now.value - loopStart.value) % lengthMs.value : 0))
const takeMs = computed(() => take.value.length * PATTERN_SAMPLE_MS)

const state = computed<'idle' | 'recording' | 'loop'>(() => (recording.value ? 'recording' : samples.value.length ? 'loop' : 'idle'))

const sliderValue = computed(() => {
  if (state.value === 'recording') return current.value
  if (state.value === 'loop') return samples.value[Math.floor(elapsed.value / PATTERN_SAMPLE_MS) % samples.value.length] ?? 0

  return 0
})

defineExpose({ state, takeMs, lengthMs, restartLoop })

const fmt = (ms: number): string => `${(ms / 1000).toFixed(1)} s`

const drawn = computed(() => (state.value === 'recording' ? take.value : samples.value))

// While recording, the graph spans the longest take (at most 10 s to start with) and grows with it.
const spanMs = computed(() =>
  state.value === 'recording'
    ? Math.max(Math.min(10_000, props.maxMs), drawn.value.length * PATTERN_SAMPLE_MS)
    : Math.max((drawn.value.length - 1) * PATTERN_SAMPLE_MS, 1)
)

const xOf = (index: number): number => ((index * PATTERN_SAMPLE_MS) / spanMs.value) * 1000

const line = computed(() => drawn.value.map((value, index) => `${xOf(index).toFixed(1)},${100 - value}`).join(' '))
const area = computed(() => (drawn.value.length ? `0,100 ${line.value} ${xOf(drawn.value.length - 1).toFixed(1)},100` : ''))

const penX = computed(() => xOf(Math.max(drawn.value.length - 1, 0)) / 1000)
const headX = computed(() => (lengthMs.value ? elapsed.value / lengthMs.value : 0))
</script>

<template>
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

      <template v-if="top < 100">
        <div
          class="pointer-events-none absolute inset-x-0 top-0 border-b border-dashed border-warning/75 bg-[repeating-linear-gradient(135deg,rgb(255_255_255/0.06)_0_6px,transparent_6px_12px)]"
          :style="{ height: `calc(1.25rem + (100% - 2.5rem) * ${(100 - top) / 100})` }"
        />
        <span
          v-if="limitLabel"
          class="absolute top-1.5 left-3 text-[11.5px] text-warning"
        >{{ limitLabel }}</span>
      </template>

      <div
        v-if="state === 'idle'"
        class="absolute inset-0 grid place-items-center px-6 pt-[20%] text-center text-[15px] text-muted"
      >
        <div>
          <b class="mb-1 block text-xl font-semibold text-highlighted">{{ idleTitle }}</b>
          {{ idleText }}
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
      :aria-valuemax="top"
      :aria-valuenow="sliderValue"
      :aria-disabled="readonly"
      class="relative w-24 shrink-0 touch-none overflow-hidden rounded-xl border well sm:w-28"
      :class="[recording ? 'cursor-grabbing border-error/70' : 'border-default', readonly ? 'opacity-60' : recording ? '' : 'cursor-grab']"
      @pointerdown="onPointerDown"
      @pointermove="onPointerMove"
      @pointerup="finish"
      @pointercancel="finish"
    >
      <div
        v-if="top < 100"
        class="pointer-events-none absolute inset-x-0 top-0 border-b border-dashed border-warning/75 bg-[repeating-linear-gradient(135deg,rgb(255_255_255/0.06)_0_6px,transparent_6px_12px)]"
        :style="{ height: `calc(${PAD + KNOB / 2}px + (100% - ${2 * PAD + KNOB}px) * ${(100 - top) / 100})` }"
      />
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
</template>
