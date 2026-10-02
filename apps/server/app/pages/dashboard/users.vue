<script setup lang="ts">
import type { AccessEntry } from '~/composables/useAdminUserAccess'
import { accessEntryName } from '~/composables/useAdminUserAccess'

// Every account that has ever signed in, with its AI access at a glance. Managing one account
// (credits, model choice, note, revoke) happens on its own page; granting access - including to
// a Discord ID that hasn't signed in yet - happens from the panel here.
const DAILY_CREDITS = 20

const { redirectIfUnauthorized } = useAdminApi()

const { data, error, refresh } = await useFetch<{ users: AccessEntry[] }>('/api/admin/users')

if (error.value) await redirectIfUnauthorized(error.value)

const users = computed(() => data.value?.users ?? [])
const query = ref('')
const filter = ref<'all' | 'access' | 'none'>('all')

const matches = (user: AccessEntry, q: string): boolean =>
  user.discordId.includes(q) || !!user.username?.toLowerCase().includes(q) || !!user.displayName?.toLowerCase().includes(q)

const filteredUsers = computed(() => {
  const q = query.value.trim().toLowerCase()

  return users.value.filter(
    (user) => (filter.value === 'all' || (filter.value === 'access') === user.hasAccess) && (!q || matches(user, q))
  )
})

const accessCount = computed(() => users.value.filter((user) => user.hasAccess).length)

const filterItems = computed(() => [
  { value: 'all' as const, label: `All · ${users.value.length}` },
  { value: 'access' as const, label: `AI access · ${accessCount.value}` },
  { value: 'none' as const, label: 'No access' }
])

// Grant panel: pick a known account without access, or paste any Discord ID.
const granting = ref(false)
const grantQuery = ref('')
const grantPick = ref<string | null>(null)
const grantNote = ref('')
const grantModel = ref(false)
const grantBusy = ref(false)

const grantCandidates = computed(() => {
  const q = grantQuery.value.trim().toLowerCase()

  return q ? users.value.filter((user) => !user.hasAccess && matches(user, q)).slice(0, 8) : []
})

const rawDiscordId = computed(() => (/^\d{15,20}$/.test(grantQuery.value.trim()) ? grantQuery.value.trim() : null))
const grantTarget = computed(() => grantPick.value ?? rawDiscordId.value)

watch(grantQuery, () => (grantPick.value = null))

const { grant } = useAdminUserAccess(refresh)

const submitGrant = async (): Promise<void> => {
  if (!grantTarget.value) return

  grantBusy.value = true

  try {
    await grant(grantTarget.value, { canSelectModel: grantModel.value, note: grantNote.value })
    granting.value = false
    grantQuery.value = ''
    grantNote.value = ''
    grantModel.value = false
  } finally {
    grantBusy.value = false
  }
}

</script>

<template>
  <div class="grid gap-4">
    <div class="flex flex-wrap items-center gap-3">
      <div class="grid flex-1 gap-0.5">
        <h1 class="text-[28px] leading-tight font-semibold text-highlighted">
          Users
        </h1>
        <p class="text-[13px] text-muted">
          Everyone who has signed in to this server
        </p>
      </div>
      <UButton
        icon="i-lucide-user-plus"
        @click="granting = !granting"
      >
        Grant AI access
      </UButton>
    </div>

    <UCard
      v-if="granting"
      :ui="{ body: 'grid gap-3.5' }"
    >
      <div class="flex items-center gap-2">
        <h2 class="flex-1 text-[17px] font-semibold text-highlighted">
          Grant AI access
        </h2>
        <UButton
          icon="i-lucide-x"
          color="neutral"
          variant="ghost"
          aria-label="Close"
          @click="granting = false"
        />
      </div>
      <UFormField label="Find a user, or paste a Discord ID">
        <UInput
          v-model="grantQuery"
          icon="i-lucide-search"
          placeholder="Name or 18-digit Discord ID"
          class="w-full"
          autofocus
        />
      </UFormField>
      <div
        v-if="grantCandidates.length"
        class="grid gap-1.5"
      >
        <button
          v-for="user in grantCandidates"
          :key="user.discordId"
          type="button"
          class="flex min-h-13.5 cursor-pointer items-center gap-3 rounded-field border px-3 text-left transition-colors"
          :class="grantPick === user.discordId ? 'border-primary bg-primary/12' : 'border-transparent bg-(--aurora-well)'"
          :aria-pressed="grantPick === user.discordId"
          @click="grantPick = user.discordId"
        >
          <UAvatar
            :src="user.avatarUrl ?? undefined"
            :alt="accessEntryName(user)"
            size="sm"
          />
          <span class="grid min-w-0 flex-1">
            <b class="truncate font-medium">{{ accessEntryName(user) }}</b>
            <small class="truncate font-mono text-[11px] text-muted">{{ user.discordId }}</small>
          </span>
          <UIcon
            v-if="grantPick === user.discordId"
            name="i-lucide-check"
            class="size-5 text-primary"
          />
        </button>
      </div>
      <UAlert
        v-else-if="rawDiscordId"
        color="neutral"
        variant="subtle"
        icon="i-lucide-info"
        title="This ID hasn't signed in yet"
        description="They'll have access the first time they do."
      />
      <p
        v-else-if="grantQuery.trim()"
        class="text-sm text-muted"
      >
        No matching users without access.
      </p>
      <UFormField
        label="Note"
        description="Only admins see this."
      >
        <UInput
          v-model="grantNote"
          placeholder="Why they have access, for example: beta tester"
          class="w-full"
          :maxlength="500"
        />
      </UFormField>
      <USwitch
        v-model="grantModel"
        label="Can pick a model"
        description="Lets them choose the AI provider."
      />
      <UButton
        icon="i-lucide-user-plus"
        class="w-fit"
        :disabled="!grantTarget"
        :loading="grantBusy"
        @click="submitGrant"
      >
        Grant access
      </UButton>
    </UCard>

    <div class="flex flex-wrap items-center gap-2.5">
      <UInput
        v-model="query"
        icon="i-lucide-search"
        placeholder="Search name or Discord ID"
        class="min-w-52 flex-1"
      />
      <AdminSegmented
        v-model="filter"
        :items="filterItems"
        label="Filter"
      />
    </div>

    <UCard :ui="{ body: 'p-0 sm:p-0' }">
      <div
        v-if="filteredUsers.length"
        class="divide-y divide-default"
      >
        <NuxtLink
          v-for="user in filteredUsers"
          :key="user.discordId"
          :to="`/dashboard/user/${user.discordId}`"
          class="grid min-h-16 grid-cols-[auto_minmax(0,1fr)_auto] items-center gap-3.5 px-4 py-2.5 transition-colors hover:bg-(--aurora-glass) md:grid-cols-[auto_minmax(0,1fr)_9rem_7rem_8rem_auto]"
        >
          <UAvatar
            :src="user.avatarUrl ?? undefined"
            :alt="accessEntryName(user)"
            size="md"
          />
          <span class="grid min-w-0">
            <b class="truncate font-medium text-highlighted">{{ accessEntryName(user) }}</b>
            <small class="truncate font-mono text-[11.5px] text-muted">{{ user.discordId }}</small>
          </span>
          <NuxtTime
            class="text-sm text-muted max-md:hidden"
            :datetime="user.lastSeenAt"
            day="numeric"
            month="short"
            hour="2-digit"
            minute="2-digit"
          />
          <UBadge
            :color="user.hasAccess ? 'success' : 'neutral'"
            variant="subtle"
            class="w-fit max-md:hidden"
          >
            {{ user.hasAccess ? 'AI access' : 'No access' }}
          </UBadge>
          <span
            v-if="user.hasAccess"
            class="grid gap-1 max-md:hidden"
          >
            <span class="font-mono text-xs tabular-nums">{{ user.credits?.remaining ?? 0 }} / {{ DAILY_CREDITS }}</span>
            <UProgress
              :model-value="Math.min(user.credits?.remaining ?? 0, DAILY_CREDITS)"
              :max="DAILY_CREDITS"
              size="xs"
            />
          </span>
          <span
            v-else
            class="text-sm text-dimmed max-md:hidden"
          >—</span>
          <UIcon
            name="i-lucide-chevron-right"
            class="size-5 text-muted"
          />
        </NuxtLink>
      </div>
      <p
        v-else
        class="py-12 text-center text-muted"
      >
        {{ users.length ? 'No users match.' : 'Nobody has signed in yet.' }}
      </p>
    </UCard>
  </div>
</template>
