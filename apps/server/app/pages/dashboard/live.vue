<script setup lang="ts">
import type { LiveRoom } from '~~/server/utils/liveRooms'

useHead({ title: 'Live · Dashboard' })

const { adminFetch, redirectIfUnauthorized } = useAdminApi()
const toast = useToast()

const { data, error, refresh } = await useFetch<{ rooms: LiveRoom[] }>('/api/admin/rooms')

if (error.value) await redirectIfUnauthorized(error.value)

// Online hosts first, the busiest on top.
const rooms = computed(() =>
  [...(data.value?.rooms ?? [])].sort((a, b) => Number(b.online) - Number(a.online) || b.viewers - a.viewers)
)

const totals = computed(() => ({
  hosts: rooms.value.filter((room) => room.online).length,
  viewers: rooms.value.reduce((sum, room) => sum + room.viewers, 0)
}))

useAdminSocket<{ type: string }>(
  (message) => {
    if (message.type === 'rooms-changed') void refresh()
  },
  () => void refresh()
)

const confirmingStop = ref<string | null>(null)
const busy = ref(false)

const stop = async (room: LiveRoom): Promise<void> => {
  busy.value = true

  try {
    await adminFetch(`/api/admin/rooms/${room.discordId}/stop`, { method: 'POST' })
    toast.add({ title: `Stopped everything for ${room.name ?? room.discordId}`, icon: 'i-lucide-octagon-x', color: 'warning' })
  } finally {
    busy.value = false
    confirmingStop.value = null
  }
}
</script>

<template>
  <div class="grid gap-4">
    <div class="grid gap-0.5">
      <h1 class="text-[28px] leading-tight font-semibold text-highlighted">
        Live
      </h1>
      <p class="text-[13px] text-muted">
        {{ totals.hosts }} {{ totals.hosts === 1 ? 'host' : 'hosts' }} online · {{ totals.viewers }} {{ totals.viewers === 1 ? 'viewer' : 'viewers' }} on share pages
      </p>
    </div>

    <UCard :ui="{ body: 'p-0 sm:p-0' }">
      <div
        v-if="rooms.length"
        class="divide-y divide-default"
      >
        <div
          v-for="room in rooms"
          :key="room.discordId"
          class="grid gap-2 px-4 py-3"
        >
          <div class="grid grid-cols-[auto_minmax(0,1fr)_auto] items-center gap-3.5 md:grid-cols-[auto_minmax(0,1fr)_8rem_9rem_auto]">
            <UAvatar
              :src="room.avatar ?? undefined"
              :alt="room.name ?? room.discordId"
              size="md"
            />
            <NuxtLink
              :to="`/dashboard/user/${room.discordId}`"
              class="grid min-w-0"
            >
              <b class="truncate font-medium text-highlighted hover:underline">{{ room.name ?? room.discordId }}</b>
              <small class="truncate text-[12px] text-muted">
                <template v-if="room.online && room.onlineSince">
                  Online since <NuxtTime
                    :datetime="room.onlineSince"
                    hour="2-digit"
                    minute="2-digit"
                  />
                </template>
                <template v-else>Host offline, viewers waiting</template>
                <template v-if="room.lastActionAt">
                  · last action <NuxtTime
                    :datetime="room.lastActionAt"
                    relative
                  />
                </template>
              </small>
            </NuxtLink>
            <span class="text-sm tabular-nums max-md:hidden">
              {{ room.viewers }} {{ room.viewers === 1 ? 'viewer' : 'viewers' }}
              <small class="block text-xs text-muted">{{ room.signedInViewers }} signed in</small>
            </span>
            <span class="flex flex-wrap gap-1.5 max-md:hidden">
              <UBadge
                :color="room.online ? 'success' : 'neutral'"
                variant="subtle"
              >
                {{ room.online ? 'Online' : 'Offline' }}
              </UBadge>
              <UBadge
                v-if="room.paused"
                color="warning"
                variant="subtle"
              >
                Stopped
              </UBadge>
            </span>
            <span class="flex items-center gap-1">
              <UButton
                :to="`/s/${room.shareCode}`"
                target="_blank"
                icon="i-lucide-external-link"
                color="neutral"
                variant="ghost"
                aria-label="Open share page"
              />
              <UButton
                v-if="room.online && !room.paused"
                icon="i-lucide-octagon-x"
                color="error"
                variant="soft"
                @click="confirmingStop = room.discordId"
              >
                Stop
              </UButton>
            </span>
          </div>

          <div
            v-if="confirmingStop === room.discordId"
            class="flex flex-wrap items-center gap-2 rounded-field border border-error/40 bg-error/10 p-3 text-sm"
          >
            <span class="flex-1">Stop everything for {{ room.name ?? 'this host' }}? Their toys, shockers and controls stop until they resume.</span>
            <UButton
              size="md"
              color="neutral"
              variant="subtle"
              @click="confirmingStop = null"
            >
              Cancel
            </UButton>
            <UButton
              size="md"
              color="error"
              :loading="busy"
              @click="stop(room)"
            >
              Stop everything
            </UButton>
          </div>
        </div>
      </div>
      <p
        v-else
        class="py-12 text-center text-muted"
      >
        Nobody is live right now.
      </p>
    </UCard>
  </div>
</template>
