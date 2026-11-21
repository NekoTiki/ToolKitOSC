<script setup lang="ts">
import type { NavigationMenuItem } from '@nuxt/ui'

import type { UserStatsResponse } from '~/types/adminStats'

// One account: the header and the section links here, each section a subpage (user/[discordId]/*)
// that gets the data fetched here as props.
const route = useRoute()
const discordId = route.params.discordId as string

const { redirectIfUnauthorized } = useAdminApi()

// Same SSR reasoning as dashboard/stats.vue - useFetch (not adminFetch/onMounted) so the initial
// HTML an admin gets already has real data in it. `days` is the Usage subpage's date range.
const days = ref(14)
const { data: userStats, error: statsError, refresh } = await useFetch<UserStatsResponse>(`/api/admin/users/${discordId}/stats`, { query: { days } })

const notFound = computed(() => (statsError.value as { statusCode?: number })?.statusCode === 404)

if (statsError.value && !notFound.value) await redirectIfUnauthorized(statsError.value)

const name = computed(() => userStats.value?.user.displayName || userStats.value?.user.username || discordId)

useHead({ title: () => `${name.value} · Dashboard` })

const onlineFor = useOnlineFor()

const base = `/dashboard/user/${discordId}`
// `active` set by hand: the Account link is the parent path, which every subpage starts with.
const sections = computed<NavigationMenuItem[]>(() =>
  [
    { label: 'Account', icon: 'i-lucide-user-cog', to: base },
    { label: 'Add-ons', icon: 'i-lucide-puzzle', to: `${base}/addons` },
    { label: 'Usage', icon: 'i-lucide-chart-column', to: `${base}/usage` },
    { label: 'Admin log', icon: 'i-lucide-scroll-text', to: `${base}/log` }
  ].map((item) => ({ ...item, active: route.path.replace(/\/$/, '') === item.to }))
)
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
              <template v-if="userStats.onlineSince">
                · <span class="text-success">online for {{ onlineFor(userStats.onlineSince) }}</span>
              </template>
              <template v-else>
                · last seen <NuxtTime
                  :datetime="userStats.user.lastSeenAt"
                  relative
                />
              </template>
              <template v-if="userStats.user.appVersion">
                · app {{ userStats.user.appVersion }}{{ userStats.user.os ? ` on ${userStats.user.os}` : '' }}
              </template>
            </template>
          </p>
        </div>
      </div>

      <!-- Scrolls sideways on a phone, never up and down. -->
      <div class="overflow-x-auto overflow-y-hidden">
        <UNavigationMenu
          :items="sections"
          variant="link"
          highlight
          aria-label="User sections"
          class="w-max min-w-full border-b border-default"
        />
      </div>

      <NuxtPage
        v-if="userStats"
        v-model:days="days"
        :discord-id="discordId"
        :user-stats="userStats"
        @changed="refresh()"
      />
    </template>
  </div>
</template>
