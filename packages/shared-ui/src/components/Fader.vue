<script setup lang="ts">
// The house fader: a well with a fill and a big square knob, dragged along a straight track (far
// easier to control with a shaky VR pointer than a dial). Used by the Slider and Toy tiles and the
// Shocker's viewer pick. It scales with its height: set `--fader-h` on it or any parent (default
// the tile's control height) and the knob, the corners and the grip follow.
//
// It's a plain v-model: `update:modelValue` on every move, already snapped to `step` and kept within
// the allowed range, and `change` once when a drag ends or a key is pressed, for callers that only
// want to commit the final value.
import { computed, ref } from 'vue'

const props = withDefaults(
  defineProps<{
    modelValue: number
    label: string
    min?: number
    max?: number
    step?: number
    // The part of the track the knob may use, on the same scale. The track still reads min-max so the
    // number shown is the real value; the parts outside are shaded and the knob can't enter them.
    rangeMin?: number
    rangeMax?: number
    // Read out instead of the bare number by screen readers ("1.5 seconds").
    valueText?: string
    color?: 'secondary' | 'error'
  }>(),
  { min: 0, max: 100, step: 1, rangeMin: undefined, rangeMax: undefined, valueText: undefined, color: 'secondary' }
)

const emit = defineEmits<{
  (e: 'update:modelValue', value: number): void
  (e: 'change', value: number): void
}>()

// Arrow keys nudge by 5% of the scale, at least one step.
const KEY_SHARE = 0.05

const span = computed(() => Math.max(props.max - props.min, Number.EPSILON))
const low = computed(() => Math.min(props.max, Math.max(props.min, props.rangeMin ?? props.min)))
const high = computed(() => Math.max(low.value, Math.min(props.max, props.rangeMax ?? props.max)))

const fraction = (value: number): number => Math.min(1, Math.max(0, (value - props.min) / span.value))

// Snapped to the step grid from `min`, then kept within the allowed range. The toFixed drops float
// noise from fractional steps (0.1 + 0.2).
const snap = (value: number): number => {
  const snapped = Math.round((value - props.min) / props.step) * props.step + props.min

  return Number(Math.min(high.value, Math.max(low.value, snapped)).toFixed(6))
}

const trackRef = ref<HTMLElement>()

// The knob's center travels between half the track height from each end, so the knob never sticks
// out past the track - see the template's `--k`.
const valueAt = (event: PointerEvent): number => {
  const rect = trackRef.value!.getBoundingClientRect()
  const radius = rect.height / 2

  return snap(props.min + ((event.clientX - rect.left - radius) / Math.max(rect.width - 2 * radius, 1)) * span.value)
}

// Pointer capture keeps the drag going when the pointer leaves the track; `change` always carries
// where it was released.
const onPointerDown = (event: PointerEvent): void => {
  // Left button only - right-click is reserved for context menus.
  if (event.button !== 0 || !trackRef.value) return

  event.preventDefault()
  trackRef.value.setPointerCapture(event.pointerId)
  emit('update:modelValue', valueAt(event))

  const track = trackRef.value
  const onMove = (e: PointerEvent): void => emit('update:modelValue', valueAt(e))
  const onUp = (e: PointerEvent): void => {
    const value = valueAt(e)

    emit('update:modelValue', value)
    emit('change', value)
    track.removeEventListener('pointermove', onMove)
    track.removeEventListener('pointerup', onUp)
    track.removeEventListener('pointercancel', onUp)
  }

  track.addEventListener('pointermove', onMove)
  track.addEventListener('pointerup', onUp)
  track.addEventListener('pointercancel', onUp)
}

const onKeydown = (event: KeyboardEvent): void => {
  const direction = { ArrowRight: 1, ArrowUp: 1, ArrowLeft: -1, ArrowDown: -1 }[event.key]

  if (direction === undefined) return

  event.preventDefault()

  const value = snap(props.modelValue + direction * Math.max(props.step, span.value * KEY_SHARE))

  emit('update:modelValue', value)
  emit('change', value)
}
</script>

<template>
  <!-- Nested like the OFF/ON switch: the fill and the knob sit 4px inside the track, with corners
  4px tighter than the track's, so every edge runs parallel. --k is the knob's size; its left edge
  travels from 4px to (100% - 4px - --k), so its center stays half a track height from each end
  (see valueAt). The fill always ends at the knob's far edge - the knob caps it, no seam. -->
  <div
    ref="trackRef"
    role="slider"
    tabindex="0"
    :aria-label="label"
    :aria-valuemin="low"
    :aria-valuemax="high"
    :aria-valuenow="modelValue"
    :aria-valuetext="valueText"
    class="fader relative cursor-grab touch-none overflow-hidden well active:cursor-grabbing"
    @pointerdown="onPointerDown"
    @keydown="onKeydown"
    @click.stop
  >
    <div
      class="fader-inner absolute inset-y-1 left-1 bg-linear-to-r"
      :class="color === 'error' ? 'from-error/30 to-error/85' : 'from-secondary/30 to-secondary/85'"
      :style="{ width: `calc(var(--k) + (100% - 0.5rem - var(--k)) * ${fraction(modelValue)})` }"
    />
    <!-- Out-of-range parts of the track, measured along the same line the knob's center travels. -->
    <div
      v-if="low > min"
      class="pointer-events-none absolute inset-y-0 left-0 bg-black/35 bg-[repeating-linear-gradient(135deg,transparent_0_6px,rgb(255_255_255/0.08)_6px_12px)]"
      :style="{ width: `calc(0.25rem + var(--k) / 2 + (100% - 0.5rem - var(--k)) * ${fraction(low)})` }"
    />
    <div
      v-if="high < max"
      class="pointer-events-none absolute inset-y-0 right-0 bg-black/35 bg-[repeating-linear-gradient(135deg,transparent_0_6px,rgb(255_255_255/0.08)_6px_12px)]"
      :style="{ width: `calc(0.25rem + var(--k) / 2 + (100% - 0.5rem - var(--k)) * ${1 - fraction(high)})` }"
    />
    <div
      class="fader-inner pointer-events-none absolute top-1 flex size-(--k) items-center justify-center gap-1 bg-white shadow-[0_2px_10px_rgb(0_0_0/0.35)]"
      :style="{ left: `calc(0.25rem + (100% - 0.5rem - var(--k)) * ${fraction(modelValue)})` }"
    >
      <!-- Grip lines: says "drag me" without relying on a hover state. -->
      <span class="h-[34%] w-0.5 rounded-full bg-black/20" />
      <span class="h-[34%] w-0.5 rounded-full bg-black/20" />
    </div>
  </div>
</template>

<style scoped>
/* Everything follows the height: a shrunk fader keeps the full-size one's proportions, with its
   corners capped at the field radius. */
.fader {
  --h: var(--fader-h, var(--tile-control));
  --k: calc(var(--h) - 0.5rem);
  --r: min(var(--radius-field), calc(var(--h) * 0.3));

  height: var(--h);
  border-radius: var(--r);
}

.fader-inner {
  border-radius: calc(var(--r) - 0.25rem);
}
</style>
