<script setup lang="ts">
import type { ControlStatsResult, StatsResult } from '~/types/adminStats'
import { DAY_RANGES } from '~/types/adminStats'

interface UserRow {
  discordId: string
  username: string | null
  displayName: string | null
  avatarUrl: string | null
  lastSeenAt: string
}

interface UserStatsResponse {
  user: UserRow
  ai: StatsResult
  controls: ControlStatsResult
}

const route = useRoute()
const discordId = route.params.discordId as string

const { redirectIfUnauthorized } = useAdminApi()

// Same SSR reasoning as dashboard/stats.vue - useFetch (not adminFetch/onMounted) so the initial
// HTML an admin gets already has real data in it.
const days = ref(14)
const { data: userStats, error: statsError } = await useFetch<UserStatsResponse>(`/api/admin/users/${discordId}/stats`, { query: { days } })

const notFound = computed(() => (statsError.value as { statusCode?: number })?.statusCode === 404)

if (statsError.value && !notFound.value) await redirectIfUnauthorized(statsError.value)

const name = computed(() => userStats.value?.user.displayName || userStats.value?.user.username || discordId)

useHead({ title: () => `${name.value} · Dashboard` })
</script>

<template>
  <div class="grid gap-4.5">
    <nav
      aria-label="Breadcrumb"
      class="flex items-center gap-1.5 text-[12.5px] text-muted"
    >
      <NuxtLink
        to="/dashboard/users"
        class="underline decoration-(--ui-border-accented) underline-offset-3 hover:text-default"
      >
        Users
      </NuxtLink>
      <span aria-hidden="true">/</span>
      <span class="truncate">{{ name }}</span>
    </nav>

    <UCard v-if="notFound">
      <p class="py-8 text-center text-sm text-muted">
        Unknown user.
      </p>
    </UCard>

    <template v-else>
      <div class="flex items-center gap-3.5">
        <UAvatar
          :src="userStats?.user.avatarUrl ?? undefined"
          :alt="name"
          size="3xl"
        />
        <div class="grid min-w-0 gap-0.5">
          <h1
            class="truncate text-[28px] leading-tight font-semibold text-highlighted"
            :title="name"
          >
            {{ name }}
          </h1>
          <p class="truncate font-mono text-xs text-muted">
            {{ discordId }}<template v-if="userStats">
              · last seen <NuxtTime
                :datetime="userStats.user.lastSeenAt"
                relative
              />
            </template>
          </p>
        </div>
      </div>

      <AdminUserAccessPanel :discord-id="discordId" />

      <div class="flex flex-wrap items-center gap-2.5">
        <h2 class="flex-1 text-lg font-semibold text-highlighted">
          Usage
        </h2>
        <AdminSegmented
          v-model="days"
          :items="DAY_RANGES"
          label="Date range"
        />
      </div>

      <AdminUsageStats
        :ai="userStats?.ai"
        :controls="userStats?.controls"
        :days="days"
        daily-title="AI generations per day"
      />

      <RecentGenerationsCard
        :discord-id="discordId"
        :limit="5"
      />
    </template>
  </div>
</template>
