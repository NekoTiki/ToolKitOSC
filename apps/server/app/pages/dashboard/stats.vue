<script setup lang="ts">
import {
  ArcElement,
  BarElement,
  CategoryScale,
  Chart as ChartJS,
  Legend,
  LinearScale,
  Tooltip
} from 'chart.js'
import { Bar, Doughnut } from 'vue-chartjs'

ChartJS.register(CategoryScale, LinearScale, BarElement, ArcElement, Tooltip, Legend)

interface StatsResult {
  daily: { day: string; success: number; failure: number }[]
  byProvider: { provider: string | null; success: number; failure: number; avgDurationMs: number | null }[]
  byProfile: { profile: string | null; count: number }[]
  totalSuccess: number
  totalFailure: number
}

interface Attempt {
  id: number
  discordId: string
  displayName: string | null
  avatarName: string | null
  provider: string | null
  profile: string | null
  model: string | null
  success: boolean
  failureReason: string | null
  errorMessage: string | null
  refunded: boolean
  durationMs: number | null
  createdAt: string
}

const { adminFetch } = useAdminApi()

// Matches the server's listRecentAttempts default (server/utils/ai/generationLog.ts) - used to
// trim the list back down to a page's worth after splicing in a live-pushed row.
const ATTEMPTS_PAGE_SIZE = 25

const days = ref(14)
const stats = ref<StatsResult | null>(null)
const statsLoading = ref(true)

async function loadStats(): Promise<void> {
  statsLoading.value = true

  try {
    stats.value = await adminFetch<StatsResult>(`/api/admin/stats?days=${days.value}`)
  } finally {
    statsLoading.value = false
  }
}

watch(days, loadStats)
onMounted(loadStats)

const failuresOnly = ref(false)
const attempts = ref<Attempt[]>([])
const attemptsLoading = ref(true)
const page = ref(1)

async function loadAttempts(): Promise<void> {
  attemptsLoading.value = true

  try {
    const query = new URLSearchParams({ page: String(page.value) })

    if (failuresOnly.value) query.set('success', 'false')

    const res = await adminFetch<{ attempts: Attempt[] }>(`/api/admin/attempts?${query.toString()}`)

    attempts.value = res.attempts
  } finally {
    attemptsLoading.value = false
  }
}

watch([failuresOnly, page], loadAttempts)
onMounted(loadAttempts)

// Live updates: /admin/ws pushes the actual new row whenever a generation attempt is logged (see
// the server's generationLog.ts) - applied locally below instead of re-running the REST fetches
// above on every single event, which would mean a full aggregate re-query + a list re-fetch for
// what's usually just one more row.
interface GenerationLoggedEvent {
  type: 'generation-logged'
  attempt: Attempt
}

function applyGenerationLogged(attempt: Attempt): void {
  if (stats.value) {
    if (attempt.success) stats.value.totalSuccess++
    else stats.value.totalFailure++

    // attempt.createdAt is an ISO string (UTC) - its first 10 chars are exactly the
    // 'YYYY-MM-DD' UTC day queryStats() groups by server-side (date(created_at, 'unixepoch')).
    const day = attempt.createdAt.slice(0, 10)
    let dayEntry = stats.value.daily.find((d) => d.day === day)

    if (!dayEntry) {
      dayEntry = { day, success: 0, failure: 0 }
      stats.value.daily.push(dayEntry)
      stats.value.daily.sort((a, b) => a.day.localeCompare(b.day))
    }

    if (attempt.success) dayEntry.success++
    else dayEntry.failure++

    let providerEntry = stats.value.byProvider.find((p) => p.provider === attempt.provider)

    if (!providerEntry) {
      // avgDurationMs is deliberately left stale here (not recomputed) - an accurate running
      // average would need to know how many prior rows actually had a duration (not every failure
      // does, see queryStats' comment), which this incremental path doesn't track. It self-corrects
      // on the next reconnect-triggered reload (see connectWs' 'open' handler below).
      providerEntry = { provider: attempt.provider, success: 0, failure: 0, avgDurationMs: null }
      stats.value.byProvider.push(providerEntry)
    }

    if (attempt.success) providerEntry.success++
    else providerEntry.failure++

    let profileEntry = stats.value.byProfile.find((p) => p.profile === attempt.profile)

    if (!profileEntry) {
      profileEntry = { profile: attempt.profile, count: 0 }
      stats.value.byProfile.push(profileEntry)
    }

    profileEntry.count++
  }

  // Only meaningful to splice into the visible list when looking at the first page (where a new
  // row would actually land) and it matches the current filter - otherwise it's left alone; the
  // admin will see it by paging back to 1, and the reconnect reload below catches up regardless.
  if (page.value === 1 && (!failuresOnly.value || !attempt.success)) {
    attempts.value = [attempt, ...attempts.value].slice(0, ATTEMPTS_PAGE_SIZE)
  }
}

let ws: WebSocket | null = null
let reconnectTimeout: ReturnType<typeof setTimeout> | undefined

async function connectWs(): Promise<void> {
  try {
    // The WS upgrade itself can't go through requireUserSession the way this fetch does, so this
    // ticket - minted over an authenticated REST call - is the actual auth check (see the server's
    // adminWsTickets.ts).
    const { ticket } = await adminFetch<{ ticket: string }>('/api/admin/ws-ticket')
    const protocol = location.protocol === 'https:' ? 'wss:' : 'ws:'

    ws = new WebSocket(`${protocol}//${location.host}/admin/ws?ticket=${ticket}`)

    // Catches up on anything missed while disconnected - a real reload, not the incremental path
    // above, since there's no way to know what was missed. Runs on the very first connect too
    // (harmless, just one extra fetch layered on the immediate onMounted loads below).
    ws.addEventListener('open', () => {
      void loadStats()
      void loadAttempts()
    })

    ws.addEventListener('message', (event: MessageEvent<string>) => {
      try {
        const data = JSON.parse(event.data) as GenerationLoggedEvent

        if (data.type === 'generation-logged') applyGenerationLogged(data.attempt)
      } catch (err) {
        console.error('[admin-ws] failed to apply message:', err)
      }
    })

    ws.addEventListener('close', () => {
      ws = null
      // Best-effort reconnect - a dashboard tab left open shouldn't need a manual reload just
      // because the connection dropped once (a server restart, a network blip).
      reconnectTimeout = setTimeout(connectWs, 5000)
    })
  } catch {
    reconnectTimeout = setTimeout(connectWs, 5000)
  }
}

onMounted(connectWs)
onBeforeUnmount(() => {
  clearTimeout(reconnectTimeout)
  ws?.close()
  ws = null
})

// Which attempt rows have their error message expanded - keyed by attempt id, reset on navigation
// (not persisted), so re-opening the page always starts collapsed.
const expandedErrors = ref<Record<number, boolean>>({})

function toggleError(id: number): void {
  expandedErrors.value[id] = !expandedErrors.value[id]
}

// Reads Tailwind's own palette straight from its CSS variables (Tailwind v4 exposes every default
// color as --color-{name}-{shade}, see node_modules/tailwindcss/theme.css - imported via
// app/assets/css/main.css's `@import "tailwindcss"`) instead of hardcoding copies of them, so
// these always match whatever Tailwind actually renders rather than a hand-picked approximation.
// Canvas fillStyle can't resolve var(...) itself the way a normal DOM element's style can, so the
// value has to be read once via getComputedStyle instead of passed through as a live reference.
function tailwindColor(name: string): string {
  if (typeof document === 'undefined') return '#000'

  return getComputedStyle(document.documentElement).getPropertyValue(`--color-${name}`).trim()
}

const CHART_COLORS = {
  success: tailwindColor('green-500'),
  failure: tailwindColor('red-500'),
  providerPalette: ['indigo-500', 'green-500', 'orange-500', 'cyan-500', 'rose-500', 'purple-500'].map(tailwindColor)
}

const dailyChartData = computed(() => ({
  labels: stats.value?.daily.map((d) => d.day) ?? [],
  datasets: [
    { label: 'Success', data: stats.value?.daily.map((d) => d.success) ?? [], backgroundColor: CHART_COLORS.success },
    { label: 'Failure', data: stats.value?.daily.map((d) => d.failure) ?? [], backgroundColor: CHART_COLORS.failure }
  ]
}))

const providerChartData = computed(() => ({
  labels: stats.value?.byProvider.map((p) => p.provider ?? 'unknown') ?? [],
  datasets: [
    {
      label: 'Generations',
      data: stats.value?.byProvider.map((p) => p.success + p.failure) ?? [],
      backgroundColor: CHART_COLORS.providerPalette
    }
  ]
}))

// Toggle on the "By provider" card (see template) between total volume (the doughnut above),
// success rate, and average generation time per provider - all three read from the same
// stats.byProvider rows already fetched/kept live for the volume view, no separate query.
const providerViewMode = ref<'volume' | 'rate' | 'duration'>('volume')
const providerViewItems = [
  { label: 'Volume', value: 'volume' as const },
  { label: 'Success rate', value: 'rate' as const },
  { label: 'Avg duration', value: 'duration' as const }
]

function successRate(row: { success: number; failure: number }): number {
  const total = row.success + row.failure

  return total ? Math.round((row.success / total) * 100) : 0
}

// A stacked success%/failure% pair per provider, always summing to 100 - fills the whole bar
// (90/10, 50/50, 100/0, ...) instead of a single segment whose height alone has to carry the
// meaning, matching the same success/failure split language as the daily chart above.
const providerRateChartData = computed(() => {
  const rows = stats.value?.byProvider ?? []
  const rates = rows.map(successRate)

  return {
    labels: rows.map((p) => p.provider ?? 'unknown'),
    datasets: [
      { label: 'Success %', data: rates, backgroundColor: CHART_COLORS.success },
      { label: 'Failure %', data: rates.map((rate) => 100 - rate), backgroundColor: CHART_COLORS.failure }
    ]
  }
})

// Same per-provider color identity as the volume doughnut, so a given provider reads as the same
// color across all three views instead of switching palettes when the toggle changes.
const providerDurationChartData = computed(() => {
  const rows = stats.value?.byProvider ?? []

  return {
    labels: rows.map((p) => p.provider ?? 'unknown'),
    datasets: [
      {
        label: 'Avg duration (s)',
        data: rows.map((p) => (p.avgDurationMs === null ? 0 : Math.round(p.avgDurationMs) / 1000)),
        backgroundColor: CHART_COLORS.providerPalette
      }
    ]
  }
})

const chartOptions = { responsive: true, maintainAspectRatio: false }
// Both custom bar views below hide the legend - each only ever shows one semantic per bar (a
// success/failure split, or a single duration value), so a legend just repeats the axis label with
// no extra information, unlike the doughnut's per-slice legend or the daily chart's two genuinely
// distinct series.
const rateChartOptions = {
  responsive: true,
  maintainAspectRatio: false,
  plugins: { legend: { display: false } },
  scales: {
    x: { stacked: true },
    y: { stacked: true, min: 0, max: 100, ticks: { callback: (value: string | number) => `${value}%` } }
  }
}
const durationChartOptions = {
  responsive: true,
  maintainAspectRatio: false,
  plugins: { legend: { display: false } },
  scales: { y: { min: 0, ticks: { callback: (value: string | number) => `${value}s` } } }
}
</script>

<template>
  <div class="flex flex-col gap-4">
    <div class="flex items-center justify-between gap-2">
      <p class="text-sm font-medium">
        Last
      </p>
      <USelect
        v-model="days"
        :items="[
          { label: '7 days', value: 7 },
          { label: '14 days', value: 14 },
          { label: '30 days', value: 30 },
          { label: '90 days', value: 90 }
        ]"
        class="w-32"
      />
    </div>

    <div
      v-if="stats"
      class="flex gap-4"
    >
      <UCard class="flex-1">
        <p class="text-xs text-muted">
          Successful generations
        </p>
        <p class="text-2xl font-semibold text-success">
          {{ stats.totalSuccess }}
        </p>
      </UCard>
      <UCard class="flex-1">
        <p class="text-xs text-muted">
          Failed generations
        </p>
        <p class="text-2xl font-semibold text-error">
          {{ stats.totalFailure }}
        </p>
      </UCard>
    </div>

    <div class="grid grid-cols-1 gap-4 lg:grid-cols-2">
      <UCard>
        <!-- Matches the "By provider" card's title+toggle row height below (an empty div holding
        the same place a UTabs would), so both charts sit at the same vertical offset and both
        cards end up the same overall height instead of this one looking shorter. -->
        <div class="mb-2 flex h-8 items-center justify-between gap-2">
          <p class="text-sm font-medium">
            Generations per day
          </p>
        </div>
        <ClientOnly>
          <div class="h-64">
            <Bar
              v-if="!statsLoading"
              :data="dailyChartData"
              :options="chartOptions"
            />
          </div>
        </ClientOnly>
      </UCard>

      <UCard>
        <div class="mb-2 flex h-8 items-center justify-between gap-2">
          <p class="text-sm font-medium">
            By provider
          </p>
          <UTabs
            v-model="providerViewMode"
            :items="providerViewItems"
            size="xs"
            class="w-72"
          />
        </div>
        <ClientOnly>
          <div class="h-64">
            <Doughnut
              v-if="!statsLoading && providerViewMode === 'volume'"
              :data="providerChartData"
              :options="chartOptions"
            />
            <Bar
              v-if="!statsLoading && providerViewMode === 'rate'"
              :data="providerRateChartData"
              :options="rateChartOptions"
            />
            <Bar
              v-if="!statsLoading && providerViewMode === 'duration'"
              :data="providerDurationChartData"
              :options="durationChartOptions"
            />
          </div>
        </ClientOnly>
      </UCard>
    </div>

    <UCard>
      <div class="mb-3 flex items-center justify-between gap-2">
        <p class="text-sm font-medium">
          Recent generations
        </p>
        <UCheckbox
          v-model="failuresOnly"
          label="Failures only"
        />
      </div>

      <div
        v-if="attemptsLoading"
        class="py-8 text-center text-sm text-muted"
      >
        Loading…
      </div>

      <div
        v-else-if="!attempts.length"
        class="py-8 text-center text-sm text-muted"
      >
        Nothing to show.
      </div>

      <div
        v-else
        class="flex flex-col divide-y divide-default text-sm"
      >
        <div
          v-for="attempt in attempts"
          :key="attempt.id"
          class="flex flex-col gap-1.5 py-2"
        >
          <div class="flex flex-wrap items-center gap-2">
            <!-- A dedicated toggle button, separate from the error text itself below - so the text
            is plain, selectable content, not something wrapped in an interactive element that
            would fight a click-drag selection or a double-click-to-select-word. -->
            <UButton
              v-if="attempt.errorMessage"
              :icon="expandedErrors[attempt.id] ? 'i-lucide-chevron-down' : 'i-lucide-chevron-right'"
              size="xs"
              color="neutral"
              variant="ghost"
              square
              @click="toggleError(attempt.id)"
            />
            <UBadge
              :color="attempt.success ? 'success' : 'error'"
              variant="subtle"
            >
              {{ attempt.success ? 'OK' : (attempt.failureReason ?? 'error') }}
            </UBadge>
            <UBadge
              v-if="attempt.refunded"
              color="neutral"
              variant="subtle"
            >
              refunded
            </UBadge>
            <span class="font-medium">{{ attempt.displayName || attempt.discordId }}</span>
            <span class="text-muted">{{ attempt.avatarName }}</span>
            <span class="text-muted">{{ attempt.provider }} · {{ attempt.profile }}</span>
            <button
              v-if="attempt.errorMessage && !expandedErrors[attempt.id]"
              type="button"
              class="max-w-xs truncate text-left text-xs text-error"
              @click="toggleError(attempt.id)"
            >
              {{ attempt.errorMessage }}
            </button>
            <span class="ml-auto text-xs text-muted">
              <template v-if="attempt.durationMs !== null">{{ (attempt.durationMs / 1000).toFixed(1) }}s · </template>{{ new Date(attempt.createdAt).toLocaleString() }}
            </span>
          </div>

          <!-- Plain text, not a button - so it can be selected/copied normally. -->
          <p
            v-if="attempt.errorMessage && expandedErrors[attempt.id]"
            class="ml-8 rounded-md bg-elevated p-2 text-xs whitespace-pre-wrap break-words text-error select-text"
          >
            {{ attempt.errorMessage }}
          </p>
        </div>
      </div>

      <div class="mt-3 flex items-center justify-end gap-2">
        <UButton
          size="xs"
          variant="soft"
          icon="i-lucide-chevron-left"
          :disabled="page <= 1"
          @click="page--"
        />
        <span class="text-xs text-muted">Page {{ page }}</span>
        <UButton
          size="xs"
          variant="soft"
          icon="i-lucide-chevron-right"
          :disabled="attempts.length < 25"
          @click="page++"
        />
      </div>
    </UCard>
  </div>
</template>
