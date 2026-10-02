<script setup lang="ts">
import type { ControlStatsResult, StatsResult } from '~/types/adminStats'
import { DAY_RANGES } from '~/types/adminStats'
import type { Attempt } from '~/types/aiAttempts'

useHead({ title: 'Overview · Dashboard' })

const { redirectIfUnauthorized } = useAdminApi()

// useFetch (not adminFetch/onMounted) so these actually run during SSR - the middleware only
// checks "logged in" (see middleware/admin.ts), the real admin check is each request's own
// response, and awaiting these here means the initial HTML an admin gets already has real charts/
// rows in it instead of a guaranteed-empty shell that fills in after hydration. `query` takes refs
// directly - useFetch already re-fetches on its own whenever one of them changes, so there's no
// need for a manual watch(...) triggering the same reload by hand.
const days = ref(14)
const {
  data: stats,
  error: statsError,
  refresh: refreshStats
} = await useFetch<StatsResult>('/api/admin/stats', { query: { days } })

if (statsError.value) await redirectIfUnauthorized(statsError.value)

// Same `days` range as the AI stats above - one date-range selector drives both sections.
const { data: controlStats, error: controlStatsError } = await useFetch<ControlStatsResult>('/api/admin/control-stats', { query: { days } })

if (controlStatsError.value) await redirectIfUnauthorized(controlStatsError.value)

// RecentGenerationsCard (see the template below) owns fetching/live-updating the attempts list
// itself - all this page adds on top is keeping its own chart aggregates (below) in sync with the
// exact same live events, via the card's 'generation-logged' emit.
function updateStatsFromAttempt(attempt: Attempt): void {
  if (!stats.value) return

  if (attempt.success) stats.value.totalSuccess++
  else stats.value.totalFailure++

  // attempt.createdAt is an ISO string (UTC) - its first 10 chars are exactly the 'YYYY-MM-DD' UTC
  // day queryStats() groups by server-side (date(created_at, 'unixepoch')).
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
    // avgDurationMs is deliberately left stale here (not recomputed) - an accurate running average
    // would need to know how many prior rows actually had a duration (not every failure does, see
    // queryStats' comment), which this incremental path doesn't track. It self-corrects on the
    // next reconnect-triggered reload (see the template's @reconnected="refreshStats").
    providerEntry = { provider: attempt.provider, providerLabel: attempt.providerLabel, success: 0, failure: 0, avgDurationMs: null }
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
</script>

<template>
  <div class="grid gap-4.5">
    <div class="flex flex-wrap items-center gap-3">
      <div class="grid min-w-0 flex-1 basis-48 gap-0.5">
        <h1 class="text-[28px] leading-tight font-semibold text-highlighted">
          Overview
        </h1>
        <p class="text-[13px] text-muted">
          All hosts on this server
        </p>
      </div>
      <AdminSegmented
        v-model="days"
        :items="DAY_RANGES"
        label="Date range"
      />
    </div>

    <AdminUsageStats
      :ai="stats"
      :controls="controlStats"
      :days="days"
      daily-title="Generations per day"
    />

    <RecentGenerationsCard
      @generation-logged="updateStatsFromAttempt"
      @reconnected="refreshStats"
    />
  </div>
</template>
