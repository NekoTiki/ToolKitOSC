<script setup lang="ts">
import type { IntifacePatternControl } from '@toolkitosc/shared-ui'
import { DEFAULT_VIEWER_PATTERN_OPTIONS, limitRange, PatternRecorderBase, useIntifacePatternControl, VIEWER_PATTERN_ID } from '@toolkitosc/shared-ui'

// "Make a pattern" on the share page: a viewer records a pattern for a Toy Pattern control with the
// slider, the toy follows them live, and letting go plays it in a loop for everyone. It isn't saved.
// On a phone it's its own page; on a desktop a panel over the right side of the tiles.
const props = defineProps<{
  control: IntifacePatternControl
  hostName: string
  // The viewer pattern playing on this control is this viewer's.
  mine: boolean
}>()

const emit = defineEmits<{
  // While recording, at most every LIVE_MS (0-1).
  (e: 'live', level: number): void
  (e: 'send', samples: number[]): void
  (e: 'stop'): void
  (e: 'close'): void
}>()

// Live values go out at most this often: 10 a second stays well under the relay's rate limit.
const LIVE_MS = 100

const recorder = useTemplateRef<InstanceType<typeof PatternRecorderBase>>('recorder')
const samples = ref<number[]>([])
const note = ref('')

const options = computed(() => ({ ...DEFAULT_VIEWER_PATTERN_OPTIONS, ...props.control.viewerPatterns }))

// The control's own cap and the active profile's max, whichever is lower.
const top = computed(() => Math.round(Math.min(options.value.maxIntensity, limitRange(props.control.limits).max) * 100))

let lastLive = 0

const onLive = (value: number): void => {
  const at = Date.now()

  // The last value (0, when recording stops) always goes out.
  if (value !== 0 && at - lastLive < LIVE_MS) return

  lastLive = at
  emit('live', value / 100)
}

const onStart = (): void => {
  note.value = ''
  lastLive = 0
}

const onFinish = (kept: boolean): void => {
  if (!kept) note.value = 'Too short. Hold the slider for at least 1 s.'
  else emit('send', samples.value)
}

const { controlValue, viewerPatterns } = useIntifacePatternControl()

const playing = computed(() => props.mine && controlValue.value.get(props.control.id) === VIEWER_PATTERN_ID)

// The playhead lines up with the toy once the host starts playing it.
watch(playing, (value) => value && recorder.value?.restartLoop())

const now = useNow({ interval: 1000 })

const endsIn = computed(() => {
  const endsAt = viewerPatterns.value.get(props.control.id)?.endsAt

  if (!playing.value || !endsAt) return null

  const seconds = Math.max(0, Math.round((endsAt - now.value.getTime()) / 1000))

  return `${Math.floor(seconds / 60)}:${String(seconds % 60).padStart(2, '0')}`
})

const fmt = (ms: number): string => `${(ms / 1000).toFixed(1)} s`

const state = computed(() => recorder.value?.state ?? 'idle')

const stopsAfterText = computed(() => {
  const ms = options.value.stopsAfterMs

  return ms ? `for up to ${ms / 60_000} min` : 'until someone stops it'
})

const status = computed(() => {
  if (state.value === 'recording') {
    return { tone: 'error', title: `Recording · ${fmt(recorder.value?.takeMs ?? 0)} of ${fmt(options.value.maxMs)}`, sub: 'The toy follows you right now' }
  }
  if (playing.value) {
    return {
      tone: 'secondary',
      title: 'Your pattern is playing',
      sub: `Repeats every ${fmt(recorder.value?.lengthMs ?? 0)}${endsIn.value ? ` · stops in ${endsIn.value}` : ''}`
    }
  }
  if (state.value === 'loop') return { tone: 'primary', title: 'Your pattern stopped', sub: 'Grab the slider to record a new one' }

  return { tone: 'primary', title: 'Ready', sub: `Up to ${fmt(options.value.maxMs)}, then it repeats` }
})

const hint = computed(() => {
  if (note.value) return note.value
  if (state.value === 'recording') return 'Let go to stop recording'
  if (state.value === 'loop') return 'Want another try? Grab the slider again.'

  return ''
})
</script>

<template>
  <aside
    aria-label="Make a pattern"
    class="flex min-h-0 flex-col gap-3 bg-(--ui-bg) p-4 lg:rounded-2xl lg:border lg:border-default lg:p-4.5 lg:shadow-[-18px_0_40px_rgba(0,0,0,.45)]"
  >
    <div class="flex items-center gap-2.5">
      <UButton
        icon="i-lucide-chevron-left"
        color="neutral"
        variant="subtle"
        size="xl"
        class="lg:hidden"
        aria-label="Back to the controls"
        @click="emit('close')"
      />
      <div class="min-w-0 flex-1">
        <h2 class="text-xl font-semibold text-highlighted">
          Make a pattern
        </h2>
        <p class="truncate text-[13px] text-muted">
          For {{ hostName }}'s <b class="text-default">{{ control.name }}</b>
        </p>
      </div>
      <UButton
        icon="i-lucide-x"
        color="neutral"
        variant="ghost"
        size="xl"
        class="max-lg:hidden"
        aria-label="Close"
        @click="emit('close')"
      />
    </div>

    <div
      class="flex items-center gap-2.5 rounded-xl border px-3.5 py-2.5"
      :class="{
        'border-error/45 bg-error/12': status.tone === 'error',
        'border-secondary/40 bg-secondary/10': status.tone === 'secondary',
        'border-primary/30 bg-primary/10': status.tone === 'primary'
      }"
    >
      <span
        v-if="state === 'recording'"
        class="size-2.5 shrink-0 animate-pulse rounded-full bg-error"
      />
      <div class="min-w-0 flex-1">
        <b class="block text-[15.5px] font-semibold text-highlighted tabular-nums">{{ status.title }}</b>
        <small class="text-[12.5px] text-muted tabular-nums">{{ status.sub }}</small>
      </div>
    </div>

    <PatternRecorderBase
      ref="recorder"
      v-model:samples="samples"
      :max-ms="options.maxMs"
      :max-value="top"
      :limit-label="top < 100 ? `Above ${hostName}'s limit (${top}%)` : undefined"
      idle-title="Hold the slider and move it"
      idle-text="Click and drag, or point and hold the trigger in VR. The toy follows you live. Let go and what you did repeats."
      class="min-h-80"
      @live="onLive"
      @start="onStart"
      @finish="onFinish"
    />

    <p
      class="min-h-5 text-center text-[13.5px]"
      :class="note ? 'text-warning' : 'text-muted'"
    >
      {{ hint }}
    </p>

    <UButton
      v-if="playing"
      icon="i-lucide-square"
      color="error"
      variant="outline"
      size="xl"
      block
      @click="emit('stop')"
    >
      Stop my pattern
    </UButton>
    <p
      v-else
      class="grid min-h-12 place-items-center text-center text-[12.5px] text-dimmed"
    >
      It isn't saved. It repeats {{ stopsAfterText }}, or until someone picks another pattern.
    </p>
  </aside>
</template>
