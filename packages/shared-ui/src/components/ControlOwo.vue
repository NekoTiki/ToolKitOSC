<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from 'vue'

import { useOwoControl } from '../composables/useOwoControl'
import type { ControlLimits, OwoControl, OwoControlCommand, OwoMuscle } from '../types/controls'
import { OWO_MIN_COOLDOWN_MS, OWO_MUSCLES, OWO_SENSATIONS } from '../types/controls'
import { limitRange } from '../utils/controlLimits'
import ControlBase from './ControlBase.vue'

// The OWO Skin: one side of the torso as its muscle groups. Pick a sensation, then tap a muscle and
// the wearer feels it there. The glow is what the suit really plays, sent back by the host, so
// every viewer sees every tap, and avatar touch too. It's electrical, so it has a cooldown after
// each tap and may be kept to viewers signed in with Discord.
const props = defineProps<{
  control: OwoControl
  limits?: ControlLimits
  // On the share page, for a viewer who isn't signed in. A missing Boolean prop reads as false, so
  // the host's own tile, which doesn't pass it, is never blocked.
  signedOut?: boolean
}>()

const emit = defineEmits<{ (e: 'command', command: Omit<OwoControlCommand, 'groupId' | 'controlId'>): void }>()

// Facing the wearer for the front (their right on your left), from behind for the back.
const LAYOUT = {
  front: ['PectoralR', 'PectoralL', 'AbdominalR', 'AbdominalL', 'ArmR', 'ArmL'],
  back: ['DorsalL', 'DorsalR', 'LumbarL', 'LumbarR']
} satisfies Record<'front' | 'back', OwoMuscle[]>

const { state } = useOwoControl()

const sides = computed(() => (['front', 'back'] as const).filter((side) => LAYOUT[side].some((m) => props.control.muscles.includes(m))))
const side = ref<'front' | 'back'>(sides.value[0] ?? 'front')

watch(sides, (value) => {
  if (!value.includes(side.value)) side.value = value[0] ?? 'front'
})

const sensations = computed(() => OWO_SENSATIONS.filter((s) => props.control.sensations.includes(s.id)))
const sensation = ref(sensations.value[0]?.id ?? 'tap')

watch(sensations, (value) => {
  if (!value.some((s) => s.id === sensation.value)) sensation.value = value[0]?.id ?? 'tap'
})

const cap = computed(() => Math.min(props.control.maxIntensity, limitRange(props.limits).max))
const level = (muscle: OwoMuscle): number => (state.value.playing[muscle] ?? 0) / 100
const busy = (other: 'front' | 'back'): boolean => LAYOUT[other].some((m) => level(m) > 0)
const touched = computed(() => state.value.touched.includes(side.value))
const signInNeeded = computed(() => props.control.signedInOnly && props.signedOut)

// shortcut: the cooldown shown is this viewer's own; someone else's tap isn't counted here, and the
// host drops a tap during theirs. Upgrade if the host ever sends cooldowns in 'owo-value-update'.
const waitUntil = ref(0)
const now = ref(Date.now())
let clock: ReturnType<typeof setInterval> | undefined
const waiting = computed(() => now.value < waitUntil.value)

onBeforeUnmount(() => clearInterval(clock))

const status = ref('')

const play = (muscle: OwoMuscle): void => {
  if (waiting.value || signInNeeded.value || !props.control.muscles.includes(muscle)) return

  emit('command', { type: 'owo', value: sensation.value, muscles: [muscle] })

  const label = OWO_SENSATIONS.find((s) => s.id === sensation.value)?.label ?? sensation.value
  status.value = `You played ${label} on ${OWO_MUSCLES[muscle].label.toLowerCase()}`
  waitUntil.value = Date.now() + Math.max(props.control.cooldown, OWO_MIN_COOLDOWN_MS)
  now.value = Date.now()
  clearInterval(clock)
  clock = setInterval(() => {
    now.value = Date.now()
    if (!waiting.value) clearInterval(clock)
  }, 200)
}
</script>

<template>
  <ControlBase
    :title="control.name"
    :icon="control.icon"
    kind="danger"
    fill
  >
    <template #badge>
      <span class="shrink-0 text-xs text-muted tabular-nums">up to {{ Math.round(cap * 100) }}%</span>
    </template>

    <div class="flex min-h-0 flex-1 flex-col gap-2">
      <div
        v-if="sides.length > 1"
        class="flex w-fit gap-1 self-center rounded-full bg-(--aurora-well) p-0.75"
        role="group"
        aria-label="Side"
      >
        <button
          v-for="option in sides"
          :key="option"
          type="button"
          class="relative min-h-8 cursor-pointer rounded-full px-3.5 text-xs transition-colors"
          :class="option === side ? 'bg-primary/35 text-highlighted' : 'text-muted hover:text-highlighted'"
          :aria-pressed="option === side"
          @click.stop="side = option"
        >
          {{ option === 'front' ? 'Front' : 'Back' }}
          <span
            v-if="option !== side && busy(option)"
            class="absolute top-1 right-1.5 size-1.5 rounded-full bg-warning shadow-[0_0_6px_var(--ui-warning)]"
          />
        </button>
      </div>

      <div class="relative grid min-h-0 flex-1">
        <div
          class="grid min-h-0 grid-cols-2 gap-1.5 rounded-t-[1.75rem] rounded-b-xl bg-(--aurora-well) p-2.5 transition-opacity"
          :class="{ 'opacity-55': touched || signInNeeded }"
          role="group"
          :aria-label="side === 'front' ? 'Front muscles' : 'Back muscles'"
        >
          <button
            v-for="muscle in LAYOUT[side]"
            :key="muscle"
            type="button"
            class="min-h-0 rounded-xl px-1 text-[11px] text-muted transition-[background-color,box-shadow,transform] duration-100 focus-visible:outline-2 focus-visible:outline-secondary enabled:cursor-pointer disabled:opacity-35"
            :disabled="!control.muscles.includes(muscle) || waiting || signInNeeded"
            :style="{
              backgroundColor: `color-mix(in srgb, var(--ui-warning) ${Math.round(level(muscle) * 100)}%, rgb(255 255 255 / 0.08))`,
              boxShadow: `inset 0 0 0 1px var(--ui-border), 0 0 ${Math.round(level(muscle) * 18)}px rgb(251 191 36 / ${(level(muscle) * 0.7).toFixed(2)})`,
              transform: `scale(${1 + level(muscle) * 0.06})`
            }"
            :aria-label="OWO_MUSCLES[muscle].label"
            @click.stop="play(muscle)"
          >
            {{ OWO_MUSCLES[muscle].short }}
          </button>
        </div>
        <div
          v-if="touched || signInNeeded"
          class="pointer-events-none absolute inset-x-2 bottom-2 flex flex-wrap items-center justify-center gap-1.5 rounded-xl bg-black/70 px-2 py-1 text-xs"
        >
          <template v-if="signInNeeded">
            <span class="text-muted">Sign in with Discord to use it</span>
          </template>
          <template v-else>
            <span class="rounded-full bg-warning/20 px-2 py-0.5 font-medium text-warning">Being touched</span>
            <span class="text-muted">Taps here wait until they stop</span>
          </template>
        </div>
      </div>

      <div
        v-if="sensations.length > 1"
        class="flex flex-wrap gap-1.5"
        role="group"
        aria-label="Sensation"
      >
        <button
          v-for="option in sensations"
          :key="option.id"
          type="button"
          class="min-h-8 cursor-pointer rounded-full px-3 text-xs font-semibold transition-colors"
          :class="option.id === sensation ? 'bg-primary/35 text-highlighted' : 'text-muted shadow-[inset_0_0_0_1px_var(--ui-border)] hover:text-highlighted'"
          :aria-pressed="option.id === sensation"
          @click.stop="sensation = option.id"
        >
          {{ option.label }}
        </button>
      </div>
      <small
        class="min-h-[1.4em] truncate text-xs text-muted tabular-nums"
        aria-live="polite"
      >{{ waiting ? `${status} · wait ${Math.ceil((waitUntil - now) / 1000)} s` : status || 'Tap a muscle to play it' }}</small>
    </div>
  </ControlBase>
</template>
