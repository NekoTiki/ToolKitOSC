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
  byProvider: { provider: string | null; success: number; failure: number }[]
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
  createdAt: string
}

const { adminFetch } = useAdminApi()

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

const failuresOnly = ref(true)
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

// Which attempt rows have their error message expanded - keyed by attempt id, reset on navigation
// (not persisted), so re-opening the page always starts collapsed.
const expandedErrors = ref<Record<number, boolean>>({})

function toggleError(id: number): void {
  expandedErrors.value[id] = !expandedErrors.value[id]
}

const dailyChartData = computed(() => ({
  labels: stats.value?.daily.map((d) => d.day) ?? [],
  datasets: [
    { label: 'Success', data: stats.value?.daily.map((d) => d.success) ?? [], backgroundColor: '#22c55e' },
    { label: 'Failure', data: stats.value?.daily.map((d) => d.failure) ?? [], backgroundColor: '#ef4444' }
  ]
}))

const providerChartData = computed(() => ({
  labels: stats.value?.byProvider.map((p) => p.provider ?? 'unknown') ?? [],
  datasets: [
    {
      label: 'Generations',
      data: stats.value?.byProvider.map((p) => p.success + p.failure) ?? [],
      backgroundColor: ['#6366f1', '#22c55e', '#f97316', '#06b6d4', '#e11d48', '#a855f7']
    }
  ]
}))

const chartOptions = { responsive: true, maintainAspectRatio: false }
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
        <p class="mb-2 text-sm font-medium">
          Generations per day
        </p>
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
        <p class="mb-2 text-sm font-medium">
          By provider
        </p>
        <ClientOnly>
          <div class="h-64">
            <Doughnut
              v-if="!statsLoading"
              :data="providerChartData"
              :options="chartOptions"
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
            <span class="ml-auto text-xs text-muted">{{ new Date(attempt.createdAt).toLocaleString() }}</span>
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
