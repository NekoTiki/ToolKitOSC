<script setup lang="ts">
import { ArcElement, BarElement, CategoryScale, Chart as ChartJS, Legend, LinearScale, Tooltip } from 'chart.js'
import { Bar, Doughnut } from 'vue-chartjs'

ChartJS.register(CategoryScale, LinearScale, BarElement, ArcElement, Tooltip, Legend)

interface UserRow {
  discordId: string
  username: string | null
  displayName: string | null
  avatarUrl: string | null
  lastSeenAt: string
}

interface AiStatsResult {
  daily: { day: string; success: number; failure: number }[]
  byProvider: { provider: string | null; success: number; failure: number; avgDurationMs: number | null }[]
  byProfile: { profile: string | null; count: number }[]
  totalSuccess: number
  totalFailure: number
}

interface ControlStatsResult {
  inventory: { type: string; count: number }[]
  totalControls: number
  activationByType: { type: string; count: number }[]
  activationDaily: { day: string; count: number }[]
  totalActivations: number
}

interface UserStatsResponse {
  user: UserRow
  ai: AiStatsResult
  controls: ControlStatsResult
}

const route = useRoute()
const discordId = route.params.discordId as string

const { redirectIfUnauthorized } = useAdminApi()

// Same SSR reasoning as dashboard/stats.vue - useFetch (not adminFetch/onMounted) so the initial
// HTML an admin gets already has real data in it.
const days = ref(14)
const {
  data: userStats,
  pending: statsLoading,
  error: statsError
} = await useFetch<UserStatsResponse>(`/api/admin/users/${discordId}/stats`, { query: { days } })

const notFound = computed(() => (statsError.value as { statusCode?: number })?.statusCode === 404)

if (statsError.value && !notFound.value) await redirectIfUnauthorized(statsError.value)

const CHART_COLORS = {
  success: tailwindColor('green-500'),
  failure: tailwindColor('red-500')
}

// Same treatment as every chart on /dashboard/stats - see that composable's own comment.
const chartBorder = useChartBorder()

const dailyChartData = computed(() => ({
  labels: userStats.value?.ai.daily.map((d) => d.day) ?? [],
  datasets: [
    { label: 'Success', data: userStats.value?.ai.daily.map((d) => d.success) ?? [], backgroundColor: CHART_COLORS.success, ...chartBorder },
    { label: 'Failure', data: userStats.value?.ai.daily.map((d) => d.failure) ?? [], backgroundColor: CHART_COLORS.failure, ...chartBorder }
  ]
}))

const { colorFor: controlTypeColor, labelFor: controlTypeLabel } = useControlTypeChartColors()

const controlInventoryChartData = computed(() => ({
  labels: userStats.value?.controls.inventory.map((i) => controlTypeLabel(i.type)) ?? [],
  datasets: [
    {
      label: 'Controls configured',
      data: userStats.value?.controls.inventory.map((i) => i.count) ?? [],
      backgroundColor: userStats.value?.controls.inventory.map((i) => controlTypeColor(i.type)) ?? [],
      ...chartBorder
    }
  ]
}))

const controlActivationChartData = computed(() => ({
  labels: userStats.value?.controls.activationByType.map((a) => controlTypeLabel(a.type)) ?? [],
  datasets: [
    {
      label: 'Activations',
      data: userStats.value?.controls.activationByType.map((a) => a.count) ?? [],
      backgroundColor: userStats.value?.controls.activationByType.map((a) => controlTypeColor(a.type)) ?? [],
      ...chartBorder
    }
  ]
}))

const chartOptions = { responsive: true, maintainAspectRatio: false }
</script>

<template>
  <div class="flex flex-col gap-4">
    <div class="flex items-center gap-2">
      <UButton
        to="/dashboard/users"
        icon="i-lucide-chevron-left"
        variant="ghost"
        size="xs"
      >
        Back to users
      </UButton>
    </div>

    <UCard v-if="notFound">
      <p class="py-8 text-center text-sm text-muted">
        Unknown user.
      </p>
    </UCard>

    <template v-else>
      <div class="flex items-center justify-between gap-2">
        <div class="flex items-center gap-3">
          <UAvatar
            :src="userStats?.user.avatarUrl ?? undefined"
            :alt="userStats?.user.displayName ?? discordId"
            size="lg"
          />
          <div>
            <p class="font-medium">
              {{ userStats?.user.displayName || userStats?.user.username || 'Unknown' }}
            </p>
            <p class="text-xs text-muted">
              {{ discordId }}
            </p>
          </div>
        </div>
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
        v-if="userStats"
        class="grid grid-cols-2 gap-4 lg:grid-cols-4"
      >
        <UCard>
          <p class="text-xs text-muted">
            Successful generations
          </p>
          <p class="text-2xl font-semibold text-success">
            {{ userStats.ai.totalSuccess }}
          </p>
        </UCard>
        <UCard>
          <p class="text-xs text-muted">
            Failed generations
          </p>
          <p class="text-2xl font-semibold text-error">
            {{ userStats.ai.totalFailure }}
          </p>
        </UCard>
        <UCard>
          <p class="text-xs text-muted">
            Controls configured
          </p>
          <p class="text-2xl font-semibold">
            {{ userStats.controls.totalControls }}
          </p>
        </UCard>
        <UCard>
          <p class="text-xs text-muted">
            Control activations
          </p>
          <p class="text-2xl font-semibold">
            {{ userStats.controls.totalActivations }}
          </p>
        </UCard>
      </div>

      <div class="grid grid-cols-1 gap-4 lg:grid-cols-2">
        <UCard>
          <p class="mb-2 text-sm font-medium">
            AI generations per day
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
            Controls configured, by type
          </p>
          <ClientOnly>
            <div class="h-64">
              <Doughnut
                v-if="!statsLoading"
                :data="controlInventoryChartData"
                :options="chartOptions"
              />
            </div>
          </ClientOnly>
        </UCard>

        <UCard>
          <p class="mb-2 text-sm font-medium">
            Activations, by type
          </p>
          <ClientOnly>
            <div class="h-64">
              <Doughnut
                v-if="!statsLoading"
                :data="controlActivationChartData"
                :options="chartOptions"
              />
            </div>
          </ClientOnly>
        </UCard>
      </div>

      <RecentGenerationsCard :discord-id="discordId" />
    </template>
  </div>
</template>
