<script setup lang="ts">
import type { AdminLogEntry } from '~~/server/db/adminLog'

// What admins did, newest first. On a user's page `discordId` narrows it to them, and the target
// column is left out.
const props = defineProps<{ discordId?: string; limit?: number }>()

const { redirectIfUnauthorized } = useAdminApi()

const log = useFetch<{ entries: AdminLogEntry[] }>('/api/admin/log', {
  query: { discordId: props.discordId, limit: props.limit ?? 100 }
})

defineExpose({ refresh: () => log.refresh() })

const { data, error } = await log

if (error.value) await redirectIfUnauthorized(error.value)

const ACTIONS: Record<string, { label: string; icon: string }> = {
  'grant-ai': { label: 'Gave AI access', icon: 'i-lucide-user-plus' },
  'revoke-ai': { label: 'Revoked AI access', icon: 'i-lucide-user-minus' },
  'set-model-choice': { label: 'Changed model choice', icon: 'i-lucide-cpu' },
  'add-credits': { label: 'Added credits', icon: 'i-lucide-credit-card' },
  'free-share-name': { label: 'Freed the custom link name', icon: 'i-lucide-eraser' },
  suspend: { label: 'Suspended', icon: 'i-lucide-ban' },
  'lift-suspension': { label: 'Lifted the suspension', icon: 'i-lucide-undo-2' },
  'sign-out-everywhere': { label: 'Signed out everywhere', icon: 'i-lucide-log-out' },
  'stop-everything': { label: 'Stopped everything', icon: 'i-lucide-octagon-x' },
  announce: { label: 'Sent an announcement', icon: 'i-lucide-megaphone' },
  'clear-announcement': { label: 'Cleared the announcement', icon: 'i-lucide-x' },
  'change-settings': { label: 'Changed server settings', icon: 'i-lucide-settings' },
  'set-daily-credits': { label: 'Changed daily credits', icon: 'i-lucide-credit-card' },
  'decline-ai-request': { label: 'Declined the AI request', icon: 'i-lucide-x' },
  'delete-data': { label: 'Deleted the data of', icon: 'i-lucide-trash-2' }
}

const actionOf = (action: string) => ACTIONS[action] ?? { label: action, icon: 'i-lucide-dot' }

// The detail object as short "key: value" text, for the row's second line.
const detailText = (detail: unknown): string =>
  detail && typeof detail === 'object'
    ? Object.entries(detail)
        .filter(([, value]) => value !== undefined && value !== null && value !== '')
        .map(([key, value]) => `${key}: ${value}`)
        .join(' · ')
    : ''
</script>

<template>
  <UCard :ui="{ body: 'p-0 sm:p-0' }">
    <ul
      v-if="data?.entries.length"
      class="divide-y divide-default"
    >
      <li
        v-for="entry in data.entries"
        :key="entry.id"
        class="grid grid-cols-[auto_minmax(0,1fr)_auto] items-center gap-3 px-4 py-2.5"
      >
        <UIcon
          :name="actionOf(entry.action).icon"
          class="size-5 text-muted"
        />
        <span class="grid min-w-0">
          <span class="truncate text-sm">
            <b class="font-medium">{{ entry.adminName ?? entry.adminId }}</b>
            {{ actionOf(entry.action).label.toLowerCase() }}
            <NuxtLink
              v-if="!discordId && entry.targetId"
              :to="`/dashboard/user/${entry.targetId}`"
              class="underline decoration-(--ui-border-accented) underline-offset-3"
            >{{ entry.targetName ?? entry.targetId }}</NuxtLink>
          </span>
          <small
            v-if="detailText(entry.detail)"
            class="truncate text-xs text-muted"
          >{{ detailText(entry.detail) }}</small>
        </span>
        <NuxtTime
          class="text-xs text-muted"
          :datetime="entry.createdAt"
          relative
        />
      </li>
    </ul>
    <p
      v-else
      class="py-10 text-center text-sm text-muted"
    >
      Nothing yet.
    </p>
  </UCard>
</template>
