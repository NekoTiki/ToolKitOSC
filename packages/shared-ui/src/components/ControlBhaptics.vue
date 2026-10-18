<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from 'vue'

import { useBhapticsControl } from '../composables/useBhapticsControl'
import type { BhapticsControl, BhapticsControlCommand, BhapticsPosition, ControlLimits } from '../types/controls'
import { BHAPTICS_HOLD_TICK_MS, BHAPTICS_LAYOUT } from '../types/controls'
import { limitRange } from '../utils/controlLimits'
import ControlBase from './ControlBase.vue'

// A body to touch: one side of the vest (or a Tactal, an arm...) as its grid of motors. Press and
// drag across them and the wearer feels it under your finger, or tap once for a short pulse. The
// glow is what the gear really plays, sent back by the host, so every viewer sees every drag, and
// avatar touch too. Keyboard: arrows move between motors, Space or Enter held plays one.
const props = defineProps<{
  control: BhapticsControl
  limits?: ControlLimits
}>()

const emit = defineEmits<{ (e: 'command', command: Omit<BhapticsControlCommand, 'groupId' | 'controlId'>): void }>()

// A press shorter than this, without moving, is a tap rather than a hold.
const TAP_MS = 220

const { state } = useBhapticsControl()

const position = ref<BhapticsPosition>(props.control.positions[0] ?? 'VestFront')

watch(
  () => props.control.positions,
  (positions) => {
    if (!positions.includes(position.value)) position.value = positions[0] ?? 'VestFront'
  }
)

const layout = computed(() => BHAPTICS_LAYOUT[position.value])
const cap = computed(() => Math.min(props.control.maxIntensity, limitRange(props.limits).max))

// The targets on the grid: each motor, blocks of 2×2, or the whole side.
const grid = computed(() => {
  const { motors, columns } = layout.value
  const rows = Math.ceil(motors / columns)

  if (props.control.grid === 'side') return { columns: 1, cells: [Array.from({ length: motors }, (_, m) => m)] }
  if (props.control.grid === 'motors') return { columns, cells: Array.from({ length: motors }, (_, m) => [m]) }

  const blockColumns = Math.ceil(columns / 2)
  const cells: number[][] = []

  for (let row = 0; row < rows; row += 2) {
    for (let column = 0; column < columns; column += 2) {
      cells.push([row, row + 1].flatMap((r) => [column, column + 1].filter((c) => c < columns).map((c) => r * columns + c)).filter((m) => m < motors))
    }
  }

  return { columns: blockColumns, cells }
})

const level = (position: BhapticsPosition, motors: number[]): number =>
  Math.max(0, ...motors.map((m) => state.value.playing[position]?.[m] ?? 0)) / 100

const busy = (other: BhapticsPosition): boolean => (state.value.playing[other] ?? []).some((l) => l > 0)
const touched = computed(() => state.value.touched.includes(position.value))

interface Hold {
  id: number
  position: BhapticsPosition
  // The cell under the finger, or -1 outside the grid.
  cell: number
  first: number
  moved: boolean
  started: number
  sent: boolean
}

const hold = ref<Hold | null>(null)
const status = ref('')
let ticker: ReturnType<typeof setInterval> | undefined

const send = (h: Hold, motors: number[], start = false): void =>
  emit('command', { type: 'bhaptics', value: 'hold', position: h.position, motors, holdId: h.id, ...(start && { start }) })

const tick = (): void => {
  const h = hold.value

  if (!h) return

  const elapsed = Date.now() - h.started

  if (elapsed > props.control.maxHoldMs) {
    end()
    status.value = `Stopped after ${Math.round(props.control.maxHoldMs / 1000)} s · press again`

    return
  }
  // Still might be a tap.
  if (!h.sent && !h.moved && elapsed < TAP_MS) return
  // Off the grid: nothing is sent, so the motors stop by themselves, and come back on return.
  if (h.cell < 0) return

  send(h, grid.value.cells[h.cell] ?? [], !h.sent)
  h.sent = true
}

const start = (cell: number): void => {
  end()
  hold.value = { id: Math.floor(Math.random() * 2 ** 31), position: position.value, cell, first: cell, moved: false, started: Date.now(), sent: false }
  status.value = ''
  ticker = setInterval(tick, BHAPTICS_HOLD_TICK_MS)
}

const move = (cell: number): void => {
  const h = hold.value

  if (!h || cell === h.cell) return
  if (cell !== h.first) h.moved = true
  h.cell = cell
}

const end = (): void => {
  clearInterval(ticker)
  ticker = undefined

  const h = hold.value

  if (!h) return

  hold.value = null

  if (h.sent) {
    send(h, [])
  } else if (h.first >= 0) {
    emit('command', { type: 'bhaptics', value: 'tap', position: h.position, motors: grid.value.cells[h.first] ?? [] })
  }
}

onBeforeUnmount(end)

const gridEl = ref<HTMLElement | null>(null)

const cellAt = (x: number, y: number): number => {
  const rect = gridEl.value?.getBoundingClientRect()

  if (!rect || x < rect.left || x > rect.right || y < rect.top || y > rect.bottom) return -1

  const { columns, cells } = grid.value
  const rows = Math.ceil(cells.length / columns)
  const column = Math.min(columns - 1, Math.floor(((x - rect.left) / rect.width) * columns))
  const row = Math.min(rows - 1, Math.floor(((y - rect.top) / rect.height) * rows))
  const cell = row * columns + column

  return cell < cells.length ? cell : -1
}

const onPointerDown = (event: PointerEvent): void => {
  if (event.button > 0) return
  event.preventDefault()
  ;(event.currentTarget as HTMLElement).setPointerCapture?.(event.pointerId)
  start(cellAt(event.clientX, event.clientY))
}

const onPointerMove = (event: PointerEvent): void => {
  if (hold.value) move(cellAt(event.clientX, event.clientY))
}

// Keyboard: roving focus between cells, Space or Enter held plays the focused one.
const focused = ref(0)
const keyHeld = ref(false)

const onKeyDown = (event: KeyboardEvent): void => {
  const { columns, cells } = grid.value
  const steps: Record<string, number> = { ArrowRight: 1, ArrowLeft: -1, ArrowDown: columns, ArrowUp: -columns }

  if (event.key === ' ' || event.key === 'Enter') {
    event.preventDefault()
    if (!keyHeld.value) {
      keyHeld.value = true
      start(focused.value)
    }

    return
  }

  const step = steps[event.key]

  if (step === undefined) return
  event.preventDefault()

  const next = focused.value + step
  const sameRow = Math.abs(step) !== 1 || Math.floor(next / columns) === Math.floor(focused.value / columns)

  if (next < 0 || next >= cells.length || !sameRow) return
  focused.value = next
  ;(gridEl.value?.children[next] as HTMLElement | undefined)?.focus()
  if (keyHeld.value) move(next)
}

const onKeyUp = (event: KeyboardEvent): void => {
  if ((event.key === ' ' || event.key === 'Enter') && keyHeld.value) {
    keyHeld.value = false
    end()
  }
}

const pick = (next: BhapticsPosition): void => {
  end()
  position.value = next
  focused.value = 0
}
</script>

<template>
  <ControlBase
    :title="control.name"
    :icon="control.icon"
    kind="value"
    fill
  >
    <template #badge>
      <span class="shrink-0 text-xs text-muted tabular-nums">up to {{ Math.round(cap * 100) }}%</span>
    </template>

    <div class="flex min-h-0 flex-1 flex-col gap-2">
      <div
        v-if="control.positions.length > 1"
        class="flex w-fit gap-1 self-center rounded-full bg-(--aurora-well) p-0.75"
        role="group"
        aria-label="Side"
      >
        <button
          v-for="option in control.positions"
          :key="option"
          type="button"
          class="relative min-h-8 cursor-pointer rounded-full px-3.5 text-xs transition-colors"
          :class="option === position ? 'bg-primary/35 text-highlighted' : 'text-muted hover:text-highlighted'"
          :aria-pressed="option === position"
          @click.stop="pick(option)"
        >
          {{ BHAPTICS_LAYOUT[option].label }}
          <span
            v-if="option !== position && busy(option)"
            class="absolute top-1 right-1.5 size-1.5 rounded-full bg-warning shadow-[0_0_6px_var(--ui-warning)]"
          />
        </button>
      </div>

      <div class="relative grid min-h-0 flex-1">
        <div
          ref="gridEl"
          class="grid min-h-0 touch-none gap-1.5 rounded-t-[1.75rem] rounded-b-xl bg-(--aurora-well) p-2.5 transition-opacity"
          :class="{ 'opacity-55': touched }"
          :style="{ gridTemplateColumns: `repeat(${grid.columns}, minmax(0, 1fr))` }"
          role="grid"
          :aria-label="`${layout.label}: hold Space or Enter on a motor to play it, arrow keys to move while holding`"
          @pointerdown.stop="onPointerDown"
          @pointermove="onPointerMove"
          @pointerup="end"
          @pointercancel="end"
          @lostpointercapture="end"
          @keydown="onKeyDown"
          @keyup="onKeyUp"
          @focusout="keyHeld && ((keyHeld = false), end())"
          @click.stop
        >
          <button
            v-for="(cell, index) in grid.cells"
            :key="`${position}-${index}`"
            type="button"
            class="relative min-h-0 cursor-crosshair rounded-xl transition-[background-color,box-shadow,transform] duration-100 focus-visible:outline-2 focus-visible:outline-secondary"
            :class="{ 'ring-2 ring-(--ui-text-highlighted)': hold && hold.position === position && hold.cell === index }"
            :style="{
              backgroundColor: `color-mix(in srgb, var(--ui-warning) ${Math.round(level(position, cell) * 100)}%, rgb(255 255 255 / 0.08))`,
              boxShadow: `inset 0 0 0 1px var(--ui-border), 0 0 ${Math.round(level(position, cell) * 18)}px rgb(251 191 36 / ${(level(position, cell) * 0.7).toFixed(2)})`,
              transform: `scale(${1 + level(position, cell) * 0.06})`
            }"
            :tabindex="index === focused ? 0 : -1"
            :aria-label="control.grid === 'side' ? layout.label : `Motor ${cell.join(', ')}`"
          />
        </div>
        <div
          v-if="touched"
          class="pointer-events-none absolute inset-x-2 bottom-2 flex flex-wrap items-center justify-center gap-1.5 rounded-xl bg-black/70 px-2 py-1 text-xs"
        >
          <span class="rounded-full bg-warning/20 px-2 py-0.5 font-medium text-warning">Being touched</span>
          <span class="text-muted">Taps here wait until they stop</span>
        </div>
      </div>

      <small
        class="min-h-[1.4em] truncate text-xs text-muted"
        aria-live="polite"
      >{{ status || 'Hold and drag to draw, tap for a pulse' }}</small>
    </div>
  </ControlBase>
</template>
