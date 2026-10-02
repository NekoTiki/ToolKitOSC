<script setup lang="ts">
// Successful and failed generations per day, stacked, as inline SVG so it renders during SSR and
// follows the page's own fonts. Red and green are hard to tell apart for some color-blind
// readers, so failures are also hatched, the legend carries the totals, and pointing at a day
// shows its numbers - color is never the only cue.
const props = defineProps<{ title: string; daily: { day: string; success: number; failure: number }[]; days: number }>()

const SUCCESS = '#12a877'
const FAILURE = '#e5484d'
// Drawn at the container's real width, so axis text stays at its CSS size instead of shrinking
// with a scaled viewBox. 640 until measured (SSR). Height, left axis gutter, bottom label gutter
// and top padding are fixed.
const box = useTemplateRef<HTMLElement>('box')
const { width: measured } = useElementSize(box)
const W = computed(() => Math.round(measured.value) || 640)
const H = 260
const L = 34
const B = 26
const T = 8

// The API only returns days that had generations - fill the gaps so every day in the range gets
// a slot. Days are UTC, like the server's date(created_at, 'unixepoch') grouping.
const columns = computed(() => {
  const byDay = new Map(props.daily.map((d) => [d.day, d]))
  const today = Date.now()

  return Array.from({ length: props.days }, (_, i) => {
    const date = new Date(today - (props.days - 1 - i) * 86_400_000)
    const entry = byDay.get(date.toISOString().slice(0, 10))

    return {
      label: date.toLocaleDateString('en-GB', { day: 'numeric', month: 'short', timeZone: 'UTC' }),
      success: entry?.success ?? 0,
      failure: entry?.failure ?? 0
    }
  })
})

const totals = computed(() => ({
  success: columns.value.reduce((n, c) => n + c.success, 0),
  failure: columns.value.reduce((n, c) => n + c.failure, 0)
}))

// A 1/2/5 step that gives four or five gridlines whatever the volume.
const axis = computed(() => {
  const max = Math.max(4, ...columns.value.map((c) => c.success + c.failure))
  const rough = max / 4
  const magnitude = 10 ** Math.floor(Math.log10(rough))
  const step = ([1, 2, 5, 10].find((m) => m * magnitude >= rough) ?? 10) * magnitude
  const top = Math.ceil(max / step) * step

  return { step, top, ticks: Array.from({ length: top / step + 1 }, (_, i) => i * step) }
})

const y = (value: number): number => T + (H - T - B) * (1 - value / axis.value.top)
const colWidth = computed(() => (W.value - L) / columns.value.length)
const barWidth = computed(() => Math.max(3, Math.min(28, colWidth.value - 6)))
const barX = (i: number): number => L + i * colWidth.value + (colWidth.value - barWidth.value) / 2
// Roughly one date label per 60px.
const labelEvery = computed(() => Math.ceil(columns.value.length / Math.max(2, Math.floor((W.value - L) / 60))))

const hatchId = useId()
const active = ref<number | null>(null)
const tipLeft = computed(() => (active.value === null ? 0 : ((L + (active.value + 0.5) * colWidth.value) / W.value) * 100))
</script>

<template>
  <UCard :ui="{ body: 'grid grid-cols-[minmax(0,1fr)] gap-3' }">
    <div class="flex flex-wrap items-center gap-2.5">
      <h2 class="min-w-0 flex-1 text-base font-semibold text-highlighted">
        {{ title }}
      </h2>
      <div class="flex flex-wrap gap-3.5 text-[12.5px] text-muted">
        <span class="flex items-center gap-1.5">
          <i
            class="size-3 rounded-[3px]"
            :style="{ background: SUCCESS }"
          />Successful · {{ totals.success.toLocaleString('en-US') }}
        </span>
        <span class="flex items-center gap-1.5">
          <i
            class="size-3 rounded-[3px]"
            :style="{ background: `repeating-linear-gradient(45deg, ${FAILURE} 0 3px, #8c2a2d 3px 5px)` }"
          />Failed · {{ totals.failure.toLocaleString('en-US') }}
        </span>
      </div>
    </div>

    <div
      ref="box"
      class="relative min-w-0"
      @pointerleave="active = null"
    >
      <svg
        :viewBox="`0 0 ${W} ${H}`"
        class="block h-auto w-full overflow-visible"
        role="img"
        :aria-label="`${title}: ${totals.success} successful, ${totals.failure} failed`"
      >
        <defs>
          <pattern
            :id="hatchId"
            width="6"
            height="6"
            patternUnits="userSpaceOnUse"
            patternTransform="rotate(45)"
          >
            <rect
              width="6"
              height="6"
              :fill="FAILURE"
            />
            <line
              x1="0"
              y1="0"
              x2="0"
              y2="6"
              stroke="rgba(0,0,0,.35)"
              stroke-width="2.5"
            />
          </pattern>
        </defs>

        <g
          v-for="tick in axis.ticks"
          :key="tick"
        >
          <line
            :x1="L"
            :x2="W"
            :y1="y(tick)"
            :y2="y(tick)"
            class="stroke-(--ui-border)"
          />
          <text
            :x="L - 6"
            :y="y(tick) + 3.5"
            text-anchor="end"
            class="fill-(--ui-text-muted) font-mono text-[10.5px]"
          >{{ tick }}</text>
        </g>

        <g
          v-for="(col, i) in columns"
          :key="i"
          @pointerenter="active = i"
          @pointerdown="active = i"
        >
          <rect
            :x="L + i * colWidth"
            :y="T"
            :width="colWidth"
            :height="H - T - B"
            :class="active === i ? 'fill-(--aurora-glass)' : 'fill-transparent'"
          />
          <rect
            v-if="col.success"
            :x="barX(i)"
            :y="y(col.success)"
            :width="barWidth"
            :height="y(0) - y(col.success)"
            rx="3"
            :fill="SUCCESS"
          />
          <rect
            v-if="col.failure"
            :x="barX(i)"
            :y="y(col.success + col.failure)"
            :width="barWidth"
            :height="Math.max(0, y(col.success) - y(col.success + col.failure) - (col.success ? 2 : 0))"
            rx="3"
            :fill="`url(#${hatchId})`"
          />
          <text
            v-if="i % labelEvery === 0"
            :x="barX(i) + barWidth / 2"
            :y="H - 8"
            text-anchor="middle"
            class="fill-(--ui-text-muted) font-mono text-[10.5px]"
          >{{ col.label }}</text>
        </g>

        <line
          :x1="L"
          :x2="W"
          :y1="y(0)"
          :y2="y(0)"
          class="stroke-(--ui-text-muted) opacity-50"
        />
      </svg>

      <div
        v-if="active !== null"
        class="pointer-events-none absolute top-0 z-10 rounded-md border border-default bg-default px-2.5 py-2 text-xs whitespace-nowrap shadow-lg"
        :style="{ left: `${tipLeft}%`, transform: tipLeft > 70 ? 'translateX(calc(-100% - 12px))' : 'translateX(12px)' }"
      >
        <b class="mb-0.5 block text-highlighted">{{ columns[active]!.label }}</b>
        <span class="flex items-center gap-1.5 text-muted"><i
          class="size-2 rounded-xs"
          :style="{ background: SUCCESS }"
        />Successful {{ columns[active]!.success }}</span>
        <span class="flex items-center gap-1.5 text-muted"><i
          class="size-2 rounded-xs"
          :style="{ background: FAILURE }"
        />Failed {{ columns[active]!.failure }}</span>
      </div>
    </div>
  </UCard>
</template>
