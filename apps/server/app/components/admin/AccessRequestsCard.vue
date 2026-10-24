<script setup lang="ts">
// People who asked for AI suggestions from the app. Approving grants access; declining lets them
// ask again in a week. New requests show up live.
interface AccessRequest {
  discordId: string
  username: string | null
  displayName: string | null
  avatarUrl: string | null
  message: string | null
  requestedAt: string
}

const emit = defineEmits<{ changed: [] }>()

const { adminFetch } = useAdminApi()
const toast = useToast()
const { grant } = useAdminUserAccess(async () => emit('changed'))

const { data, refresh } = await useFetch<{ requests: AccessRequest[] }>('/api/admin/access-requests')

useAdminSocket<{ type: string }>(
  (message) => {
    if (message.type === 'access-requested') void refresh()
  },
  () => void refresh()
)

const busy = ref<string | null>(null)

const run = async (discordId: string, action: () => Promise<void>): Promise<void> => {
  busy.value = discordId

  try {
    await action()
    await refresh()
  } finally {
    busy.value = null
  }
}

const decline = (request: AccessRequest) =>
  run(request.discordId, async () => {
    await adminFetch(`/api/admin/access-requests/${request.discordId}/decline`, { method: 'POST' })
    toast.add({ title: 'Request declined', icon: 'i-lucide-x' })
  })
</script>

<template>
  <UCard
    v-if="data?.requests.length"
    :ui="{ body: 'grid gap-3' }"
  >
    <h2 class="text-[17px] font-semibold text-highlighted">
      Asking for AI access
      <UBadge
        color="primary"
        variant="subtle"
      >
        {{ data.requests.length }}
      </UBadge>
    </h2>
    <ul class="grid gap-2">
      <li
        v-for="request in data.requests"
        :key="request.discordId"
        class="grid grid-cols-[auto_minmax(0,1fr)] items-center gap-3 rounded-field bg-(--aurora-well) p-3 sm:grid-cols-[auto_minmax(0,1fr)_auto]"
      >
        <UAvatar
          :src="request.avatarUrl ?? undefined"
          :alt="accessEntryName(request)"
          size="md"
        />
        <span class="grid min-w-0">
          <NuxtLink
            :to="`/dashboard/user/${request.discordId}`"
            class="truncate font-medium text-highlighted hover:underline"
          >{{ accessEntryName(request) }}</NuxtLink>
          <small class="truncate text-xs text-muted">
            <NuxtTime
              :datetime="request.requestedAt"
              relative
            /><template v-if="request.message"> · “{{ request.message }}”</template>
          </small>
        </span>
        <span class="flex gap-2 max-sm:col-span-2">
          <UButton
            icon="i-lucide-check"
            :loading="busy === request.discordId"
            @click="run(request.discordId, () => grant(request.discordId))"
          >
            Approve
          </UButton>
          <UButton
            icon="i-lucide-x"
            color="neutral"
            variant="subtle"
            :disabled="busy === request.discordId"
            @click="decline(request)"
          >
            Decline
          </UButton>
        </span>
      </li>
    </ul>
  </UCard>
</template>
