<script setup lang="ts">
import type { AccessEntry } from '~/composables/useAdminUserAccess'
import { accessEntryName } from '~/composables/useAdminUserAccess'

// Every AI generation, newest first, with generations still running at the top. `?user=` (from a
// user page's "See all") narrows it to one account.
useHead({ title: 'Generations · Dashboard' })

const route = useRoute()
const discordId = computed(() => (typeof route.query.user === 'string' && route.query.user) || undefined)

// Only for the name on the "this user only" filter chip.
const { data: usersResponse } = await useFetch<{ users: AccessEntry[] }>('/api/admin/users')

const userName = computed(() => {
  const entry = usersResponse.value?.users.find((user) => user.discordId === discordId.value)

  return entry ? accessEntryName(entry) : undefined
})
</script>

<template>
  <div class="grid gap-4.5">
    <div class="grid gap-0.5">
      <h1 class="text-[28px] leading-tight font-semibold text-highlighted">
        Generations
      </h1>
      <p class="text-[13px] text-muted">
        Every AI request, newest first · updates live
      </p>
    </div>
    <!-- Keyed by user, so clearing the filter starts a fresh log instead of re-filtering in place. -->
    <RecentGenerationsCard
      :key="discordId ?? 'all'"
      :discord-id="discordId"
      :user-name="userName"
      full
    />
  </div>
</template>
