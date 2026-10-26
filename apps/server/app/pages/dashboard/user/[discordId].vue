<script setup lang="ts">
import type { ControlStatsResult, StatsResult } from '~/types/adminStats'
import { DAY_RANGES } from '~/types/adminStats'

interface UserRow {
  discordId: string
  username: string | null
  displayName: string | null
  avatarUrl: string | null
  lastSeenAt: string
  suspendedUntil: string | null
  suspendReason: string | null
  appVersion: string | null
  os: string | null
}

interface UserStatsResponse {
  user: UserRow
  ai: StatsResult
  controls: ControlStatsResult
  shareLinks: { main: string | null; custom: string | null }
}

const route = useRoute()
const discordId = route.params.discordId as string

const { redirectIfUnauthorized } = useAdminApi()

// Same SSR reasoning as dashboard/stats.vue - useFetch (not adminFetch/onMounted) so the initial
// HTML an admin gets already has real data in it.
const days = ref(14)
const { data: userStats, error: statsError, refresh } = await useFetch<UserStatsResponse>(`/api/admin/users/${discordId}/stats`, { query: { days } })

const notFound = computed(() => (statsError.value as { statusCode?: number })?.statusCode === 404)

if (statsError.value && !notFound.value) await redirectIfUnauthorized(statsError.value)

const name = computed(() => userStats.value?.user.displayName || userStats.value?.user.username || discordId)

useHead({ title: () => `${name.value} · Dashboard` })

const log = useTemplateRef<{ refresh: () => Promise<void> }>('log')

// An action shows up in the log below as well.
const refreshAll = (): void => {
  void refresh()
  void log.value?.refresh()
}
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
              <template v-if="userStats.user.appVersion">
                · app {{ userStats.user.appVersion }}{{ userStats.user.os ? ` on ${userStats.user.os}` : '' }}
              </template>
            </template>
          </p>
        </div>
      </div>

      <AdminAccountPanel
        v-if="userStats"
        :discord-id="discordId"
        :suspended-until="userStats.user.suspendedUntil"
        :suspend-reason="userStats.user.suspendReason"
        @changed="refreshAll"
        @deleted="navigateTo('/dashboard/users')"
      />

      <AdminUserAccessPanel :discord-id="discordId" />

      <AdminShareLinkPanel
        v-if="userStats"
        :discord-id="discordId"
        :links="userStats.shareLinks"
        @changed="refreshAll"
      />

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

      <h2 class="text-lg font-semibold text-highlighted">
        Admin actions
      </h2>
      <AdminLogList
        ref="log"
        :discord-id="discordId"
        :limit="20"
      />
    </template>
  </div>
</template>
