<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from 'vue'

import { useVoiceFxControl } from '../composables/useVoiceFxControl'
import type { ControlLimits, VoiceFxControl, VoiceFxControlCommand, VoiceFxEffect } from '../types/controls'
import { VOICE_FX_EFFECTS, voiceFxEffect } from '../types/controls'
import { limitRange } from '../utils/controlLimits'
import ControlBase from './ControlBase.vue'
import Fader from './Fader.vue'

// A voice effect on the wearer's voice. Three kinds of the same tile: a toggle and a timed button
// that play what the control says, and all in one, where you pick the effect and the strength.
// The name on top is only a name: what plays is always written in the tile. Whether it's on, who
// turned it on and the cooldown come back from the host, so every viewer sees the same.
const props = defineProps<{
  control: VoiceFxControl
  limits?: ControlLimits
}>()

const emit = defineEmits<{ (e: 'command', command: Omit<VoiceFxControlCommand, 'groupId' | 'controlId'>): void }>()

const { controls } = useVoiceFxControl()

const state = computed(() => controls.value[props.control.id])
const on = computed(() => !!state.value?.on)
const pick = computed(() => props.control.kind === 'pick')

// All in one: what you picked. The strength stays within the profile's range.
const offered = computed(() => VOICE_FX_EFFECTS.filter((effect) => props.control.effects.includes(effect.id)))
const range = computed(() => limitRange(props.limits))
const picked = ref<VoiceFxEffect>(offered.value[0]?.id ?? props.control.effect)
const power = ref(Math.round(Math.min(Math.max(props.control.level, range.value.min), range.value.max) * 100))

watch(offered, (value) => {
  if (!value.some((effect) => effect.id === picked.value)) picked.value = value[0]?.id ?? props.control.effect
})
watch(range, ({ min, max }) => (power.value = Math.min(Math.max(power.value, Math.round(min * 100)), Math.round(max * 100))))

// What the tile says plays: the hold's own while it's on, otherwise what a press would start.
const effect = computed(() => voiceFxEffect(on.value && state.value?.effect ? state.value.effect : pick.value ? picked.value : props.control.effect))
const level = computed(() => (on.value && state.value?.level != null ? state.value.level : pick.value ? power.value / 100 : props.control.level))

// shortcut: countdowns compare the host's clock with this device's, so they're off by however
// far apart the two are. Send remaining times instead if viewers report it.
const now = ref(Date.now())
let clock: ReturnType<typeof setInterval> | undefined
const playsFor = computed(() => (on.value && state.value?.endsAt ? Math.max(0, state.value.endsAt - now.value) : 0))
const coolsFor = computed(() => (state.value?.cooldownUntil ? Math.max(0, state.value.cooldownUntil - now.value) : 0))
// Timed and still playing or cooling down: it can't be started again yet.
const busy = computed(() => playsFor.value > 0 || coolsFor.value > 0)

watch(
  () => [state.value?.endsAt, state.value?.cooldownUntil],
  () => {
    now.value = Date.now()
    clearInterval(clock)
    if (!busy.value) return

    clock = setInterval(() => {
      now.value = Date.now()
      if (!busy.value) clearInterval(clock)
    }, 200)
  },
  { immediate: true }
)

onBeforeUnmount(() => clearInterval(clock))

// How much of the play time, then of the cooldown, is left: 1 to 0, for the bar on the button.
const bar = computed(() => {
  if (playsFor.value > 0) return Math.min(1, playsFor.value / (props.control.seconds * 1000))
  if (coolsFor.value > 0 && props.control.cooldown > 0) return Math.min(1, (coolsFor.value - playsFor.value) / (props.control.cooldown * 1000))

  return 0
})

const send = (value: boolean, timed = false): void => {
  emit('command', pick.value && value ? { type: 'voice-fx', value, effect: picked.value, level: power.value / 100, timed } : { type: 'voice-fx', value })
}

const toggle = (): void => {
  // A timed one that's playing isn't a switch: it ends on its own.
  if (on.value && playsFor.value > 0) return

  send(!on.value)
}

const start = (): void => {
  if (!busy.value) send(true, true)
}

const SWITCH = 'relative flex h-(--tile-control) min-w-0 items-center overflow-hidden rounded-field well p-1 text-[calc(var(--tile-title)*0.95)] font-semibold tracking-wide'
const BUTTON =
  'relative flex h-(--tile-control) min-w-0 cursor-pointer items-center justify-center gap-2 overflow-hidden rounded-field border border-primary/55 bg-primary/15 px-3 text-[calc(var(--tile-title)*0.95)] font-semibold tracking-wide text-primary disabled:cursor-wait disabled:opacity-80'
</script>

<template>
  <!-- A toggle is one big switch, like every other toggle. The other two take a press on their
  own buttons. -->
  <control-base
    :title="control.name"
    :active="on"
    :role="control.kind === 'toggle' ? 'switch' : undefined"
    :tabindex="control.kind === 'toggle' ? 0 : undefined"
    :aria-checked="control.kind === 'toggle' ? on : undefined"
    :aria-label="control.kind === 'toggle' ? `${control.name}: ${effect.label} at ${Math.round(level * 100)}%` : undefined"
    @click="control.kind === 'toggle' && toggle()"
    @keydown.enter.space.prevent="control.kind === 'toggle' && toggle()"
  >
    <template #badge>
      <span
        class="grid size-5 shrink-0 place-items-center rounded-full bg-primary/15 text-primary"
        :title="control.kind === 'timed' ? `${effect.label} for ${control.seconds} s` : effect.label"
      >
        <UIcon
          :name="control.kind === 'timed' ? 'i-lucide-timer' : effect.icon"
          class="size-3"
        />
      </span>
    </template>

    <template #middle>
      <!-- All in one: pick the effect, then the strength. -->
      <div
        v-if="pick"
        class="grid w-full gap-1.5 [--fader-h:calc(var(--tile-control)*0.62)]"
        @click.stop
      >
        <div
          class="flex gap-1"
          role="group"
          aria-label="Effect"
        >
          <button
            v-for="option in offered"
            :key="option.id"
            type="button"
            class="flex min-h-8 min-w-0 flex-1 cursor-pointer items-center justify-center gap-1 rounded-field px-1.5 text-xs font-medium transition-colors disabled:cursor-default"
            :class="option.id === effect.id ? 'bg-primary/30 text-highlighted' : 'well text-muted hover:text-highlighted'"
            :aria-pressed="option.id === effect.id"
            :disabled="on"
            @click.stop="picked = option.id"
          >
            <UIcon
              :name="option.icon"
              class="size-3.5 shrink-0"
            />
            <span class="truncate">{{ option.label }}</span>
          </button>
        </div>
        <div class="flex items-center gap-2">
          <span class="grid w-14 shrink-0 gap-0.5 text-left">
            <span class="font-mono text-[10px] leading-none text-muted uppercase">Power</span>
            <span class="text-[calc(var(--tile-value)*0.55)] leading-none font-semibold text-highlighted tabular-nums">{{ Math.round(level * 100) }}<span class="text-[0.6em] opacity-70">%</span></span>
          </span>
          <fader
            v-model="power"
            class="min-w-0 flex-1"
            :class="{ 'pointer-events-none opacity-60': on }"
            label="Power"
            :range-min="Math.round(range.min * 100)"
            :range-max="Math.round(range.max * 100)"
            :value-text="`${power}%`"
          />
        </div>
      </div>

      <!-- Toggle and timed: what it plays, like the shocker's two readouts. -->
      <div
        v-else
        class="grid w-full grid-cols-[minmax(0,1.6fr)_minmax(0,1fr)] gap-1.5"
      >
        <div class="grid min-w-0 gap-1 rounded-field well px-2 pt-[calc(var(--tile-pad)*0.45)] pb-[calc(var(--tile-pad)*0.8)]">
          <span class="font-mono text-[10px] leading-none text-muted uppercase">Effect</span>
          <span class="truncate text-[calc(var(--tile-value)*0.5)] leading-tight font-semibold text-highlighted">{{ effect.label }}</span>
        </div>
        <div class="relative grid min-w-0 gap-1 overflow-hidden rounded-field well px-2 pt-[calc(var(--tile-pad)*0.45)] pb-[calc(var(--tile-pad)*0.8)]">
          <span class="font-mono text-[10px] leading-none text-muted uppercase">Power</span>
          <span class="truncate text-[calc(var(--tile-value)*0.5)] leading-tight font-semibold text-highlighted tabular-nums">{{ Math.round(level * 100) }}<span class="text-[0.6em] opacity-70">%</span></span>
          <span
            class="absolute bottom-[calc(var(--tile-pad)*0.3)] left-2 h-1 rounded-full bg-secondary/80"
            :style="{ width: `calc((100% - 1rem) * ${level})` }"
          />
        </div>
      </div>
    </template>

    <div class="flex w-full gap-1.5">
      <!-- OFF/ON, as on a toggle tile. On a toggle the whole tile is the switch, so this one is
      only what it looks like. -->
      <component
        :is="pick ? 'button' : 'div'"
        v-if="control.kind !== 'timed'"
        :type="pick ? 'button' : undefined"
        :class="[SWITCH, 'flex-1', { 'cursor-pointer': pick }]"
        :role="pick ? 'switch' : undefined"
        :aria-checked="pick ? on : undefined"
        :aria-label="pick ? `${effect.label} at ${Math.round(level * 100)}%, until turned off` : undefined"
        @click.stop="toggle"
      >
        <span
          class="absolute inset-y-1 left-1 w-[calc(50%-0.25rem)] rounded-[calc(var(--radius-field)-0.25rem)] transition-[translate,background-color] duration-200"
          :class="on ? 'translate-x-full bg-primary shadow-[0_0_16px_color-mix(in_oklab,var(--ui-primary)_45%,transparent)]' : 'bg-accented'"
        />
        <span
          class="relative z-10 flex flex-1 items-center justify-center transition-colors"
          :class="on ? 'text-dimmed' : 'text-highlighted'"
        >OFF</span>
        <span
          class="relative z-10 flex flex-1 items-center justify-center transition-colors"
          :class="on ? 'text-inverted' : 'text-dimmed'"
        >ON</span>
      </component>

      <!-- Timed: one press plays it for the control's length. While it plays, then cools down,
      the button counts down and the bar drains. -->
      <button
        v-if="control.kind !== 'toggle'"
        type="button"
        :class="[BUTTON, pick ? 'shrink-0' : 'flex-1']"
        :disabled="busy || (pick && on)"
        :aria-label="`${effect.label} at ${Math.round(level * 100)}% for ${control.seconds} seconds`"
        @click.stop="start"
      >
        <UIcon
          :name="playsFor > 0 ? 'i-lucide-audio-lines' : coolsFor > 0 ? 'i-lucide-hourglass' : 'i-lucide-play'"
          class="size-4 shrink-0"
        />
        <span class="tabular-nums">
          <template v-if="playsFor > 0">{{ Math.ceil(playsFor / 1000) }}s</template>
          <template v-else-if="coolsFor > 0">{{ Math.ceil(coolsFor / 1000) }}s</template>
          <template v-else>{{ pick ? '' : 'START · ' }}{{ control.seconds }}s</template>
        </span>
        <span
          v-if="busy"
          class="absolute bottom-0 left-0 h-1 bg-primary"
          :style="{ width: `${bar * 100}%` }"
        />
      </button>
    </div>
  </control-base>
</template>
