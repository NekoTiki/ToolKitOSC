<script setup lang="ts">
import EmptyState from '@renderer/components/ui/EmptyState.vue'
import FormSection from '@renderer/components/ui/FormSection.vue'
import { useAreYouSureModal } from '@renderer/composables/useAreYouSureModal'
import { useBannedClientsDb } from '@renderer/composables/useBannedClientsDb'
import { useBanViewerModal } from '@renderer/composables/useBanViewerModal'
import { formatUniqueKey, useClientsDb } from '@renderer/composables/useClientsDb'
import { useLiveQuery } from '@renderer/composables/useLiveQuery'
import type { Client } from '@renderer/db/clients.db'
import { db as clientsDb } from '@renderer/db/clients.db'
import type { Command } from '@renderer/db/commands.db'
import { db } from '@renderer/db/commands.db'
import { formatCommandValue } from '@renderer/utils/commandLog'
import { timeAgo } from '@renderer/utils/time'
import { useNow } from '@vueuse/core'
import { computed, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'

// Everyone who has ever connected to your share page: online, offline and banned in the sidebar;
// the selected viewer's details, ban state (with its reason, now shown) and recent activity in the
// main pane. Replaces the clients slideover and per-client log modal.
const RECENT_LIMIT = 10

const route = useRoute()
const router = useRouter()
const toast = useToast()
const { onlineClients } = useClientsDb()
const { isBanned, ban, unban } = useBannedClientsDb()
const { openModal: openBanModal } = useBanViewerModal()
const { openModal: confirm } = useAreYouSureModal()

const clients = useLiveQuery(
  () => null,
  () => clientsDb.clients.toArray(),
  [] as Client[]
)

const search = ref('')

// Re-renders the relative "last seen" times as they age.
const now = useNow({ interval: 30_000 })

const lastSeen = (client: Client): string | null => (client.lastSeenAt ? timeAgo(client.lastSeenAt, now.value) : null)

const isOnline = (client: Client): boolean => onlineClients.value.includes(formatUniqueKey(client.ip, client.discordId))

const filtered = computed(() => {
  const query = search.value.trim().toLowerCase()

  return clients.value
    .filter((client) => !query || client.displayName.toLowerCase().includes(query) || client.discordId?.includes(query))
    .sort((a, b) => a.displayName.localeCompare(b.displayName))
})

const sections = computed(() => [
  { key: 'online', label: 'Online', clients: filtered.value.filter((c) => !isBanned(c.ip, c.discordId) && isOnline(c)) },
  { key: 'offline', label: 'Offline', clients: filtered.value.filter((c) => !isBanned(c.ip, c.discordId) && !isOnline(c)) },
  { key: 'banned', label: 'Banned', clients: filtered.value.filter((c) => isBanned(c.ip, c.discordId)) }
])

const selectedId = computed(() => Number(route.params.clientId) || null)
const selected = computed(() => clients.value.find((client) => client.id === selectedId.value) ?? null)

// /viewers alone opens the first viewer (online ones first).
watch(
  [sections, selectedId],
  () => {
    if (selected.value || !clients.value.length) return
    const first = sections.value.flatMap((section) => section.clients)[0]
    if (first?.id !== undefined) void router.replace(`/viewers/${first.id}`)
  },
  { immediate: true }
)

const banned = computed(() => (selected.value ? isBanned(selected.value.ip, selected.value.discordId) : undefined))

// A viewer's commands are matched by Discord id when they have one, else by IP - the same rule the
// old per-client log used.
const commandsFor = (client: Client | null): Promise<{ count: number; recent: Command[] }> => {
  if (!client) return Promise.resolve({ count: 0, recent: [] })

  const range = client.discordId
    ? db.commands.where('[discordId+createdAt]').between([client.discordId, 0], [client.discordId, Infinity])
    : db.commands.where('[ip+createdAt]').between([client.ip, 0], [client.ip, Infinity])

  return Promise.all([range.count(), range.clone().reverse().limit(RECENT_LIMIT).toArray()]).then(([count, recent]) => ({ count, recent }))
}

const activity = useLiveQuery(selected, commandsFor, { count: 0, recent: [] as Command[] })

const showIp = ref(false)

watch(selectedId, () => (showIp.value = false))

const maskedIp = computed(() => {
  const ip = selected.value?.ip ?? ''

  return showIp.value ? ip : ip.replace(/[^.:]+(?=[.:][^.:]+$)|[^.:]+$/g, '•••')
})

const doBan = async (): Promise<void> => {
  const client = selected.value

  if (!client) return

  const result = await openBanModal(client)

  if (!result) return

  await ban(result.scope, result.scope === 'discord' ? client.discordId! : client.ip, result.reason)
  toast.add({ title: `${client.displayName} banned`, icon: 'i-lucide-ban' })
}

const doUnban = async (): Promise<void> => {
  const client = selected.value
  const entry = banned.value

  if (!client || entry?.id === undefined) return

  const ok = await confirm({
    title: `Unban ${client.displayName}?`,
    message: `This lifts the ban on their **${entry.scope === 'discord' ? 'Discord account' : 'IP address'}**. They can use your controls again.`,
    confirmText: 'Unban',
    danger: false
  })

  if (!ok) return

  await unban(entry.id)
  toast.add({ title: `${client.displayName} unbanned`, icon: 'i-lucide-shield-check', color: 'success' })
}

const formatDate = (timestamp: number): string => new Date(timestamp).toLocaleDateString(undefined, { day: 'numeric', month: 'short', year: 'numeric' })
</script>

<template>
  <PageLayout>
    <template #sidebar>
      <div class="flex min-h-9 shrink-0 items-center">
        <b class="text-xl font-semibold text-highlighted">Viewers</b>
      </div>
      <UInput
        v-model="search"
        icon="i-lucide-search"
        placeholder="Search viewers"
        class="shrink-0"
        :ui="{ base: 'rounded-2xl' }"
      />

      <nav
        aria-label="Viewers"
        class="-mr-1.5 flex min-h-0 flex-1 flex-col gap-1.5 overflow-y-auto pr-1.5"
      >
        <template
          v-for="section in sections"
          :key="section.key"
        >
          <div
            class="flex justify-between px-2.5 pt-2 font-mono text-[10.5px] tracking-widest uppercase"
            :class="section.key === 'banned' ? 'text-error' : 'text-muted'"
          >
            <span>{{ section.label }}</span>
            <span>{{ section.clients.length }}</span>
          </div>
          <RouterLink
            v-for="client in section.clients"
            :key="client.id"
            :to="`/viewers/${client.id}`"
            class="grid min-h-14.5 grid-cols-[auto_minmax(0,1fr)] items-center gap-x-2.5 rounded-2xl border px-2.5 py-2 transition-colors"
            :class="client.id === selectedId ? 'border-primary/50 bg-primary/14' : 'border-transparent hover:bg-(--aurora-glass)'"
          >
            <UChip
              :show="section.key === 'online'"
              color="success"
              position="bottom-right"
              inset
              class="row-span-2"
            >
              <UAvatar
                :src="client.avatar"
                :alt="client.displayName"
                size="md"
              />
            </UChip>
            <b class="truncate text-[14.5px] font-semibold text-highlighted">{{ client.displayName }}</b>
            <small class="col-start-2 truncate text-xs text-muted">
              {{ client.discordId ? 'Discord' : 'Name only' }}<template v-if="section.key !== 'online' && lastSeen(client)"> · last seen {{ lastSeen(client) }}</template>
            </small>
          </RouterLink>
        </template>
      </nav>
    </template>

    <template
      v-if="selected"
      #header
    >
      <div class="flex min-w-0 flex-1 items-center gap-3.5">
        <UAvatar
          :src="selected.avatar"
          :alt="selected.displayName"
          size="3xl"
        />
        <div class="grid min-w-0 gap-1">
          <h1
            class="truncate text-[28px] leading-tight font-semibold text-highlighted"
            :title="selected.displayName"
          >
            {{ selected.displayName }}
          </h1>
          <div class="flex flex-wrap gap-1.5 text-xs font-medium">
            <span
              v-if="banned"
              class="rounded-full bg-error/15 px-2.5 py-0.5 text-error"
            >Banned</span>
            <span
              v-else-if="isOnline(selected)"
              class="rounded-full bg-success/15 px-2.5 py-0.5 text-success"
            >Online</span>
            <span
              v-else
              class="rounded-full bg-(--aurora-well) px-2.5 py-0.5 text-muted"
            >Offline</span>
            <span
              class="rounded-full px-2.5 py-0.5"
              :class="selected.discordId ? 'bg-secondary/15 text-secondary' : 'bg-(--aurora-well) text-muted'"
            >{{ selected.discordId ? 'Discord account' : 'Name only' }}</span>
          </div>
        </div>
      </div>
      <UButton
        v-if="banned"
        icon="i-lucide-shield-check"
        color="secondary"
        variant="soft"
        @click="doUnban"
      >
        Unban
      </UButton>
      <UButton
        v-else
        icon="i-lucide-ban"
        color="error"
        variant="soft"
        @click="doBan"
      >
        Ban
      </UButton>
    </template>

    <div
      v-if="selected"
      class="grid max-w-5xl grid-cols-1 gap-3.5"
    >
      <div
        v-if="banned"
        class="flex items-center gap-3 rounded-field border border-error/40 bg-error/10 px-3.5 py-3 text-sm"
      >
        <UIcon
          name="i-lucide-ban"
          class="size-5 shrink-0 text-error"
        />
        <div class="min-w-0">
          Banned by {{ banned.scope === 'ip' ? 'IP address' : 'Discord account' }} on {{ formatDate(banned.createdAt) }}
          <small class="block text-muted">{{ banned.reason ? `Reason: ${banned.reason}` : 'No reason given' }}</small>
        </div>
      </div>

      <div class="grid grid-cols-1 items-start gap-3.5 lg:grid-cols-2">
        <FormSection title="Details">
          <dl class="grid grid-cols-[auto_minmax(0,1fr)] items-center gap-x-4.5 gap-y-2.5 text-[13.5px]">
            <dt class="text-muted">
              Discord ID
            </dt>
            <dd class="truncate font-mono">
              {{ selected.discordId ?? '—' }}
            </dd>
            <dt class="text-muted">
              IP address
            </dt>
            <dd class="flex min-w-0 items-center gap-2 font-mono">
              <span class="truncate">{{ maskedIp }}</span>
              <UButton
                size="sm"
                color="neutral"
                variant="ghost"
                @click="showIp = !showIp"
              >
                {{ showIp ? 'Hide' : 'Show' }}
              </UButton>
            </dd>
            <dt class="text-muted">
              First seen
            </dt>
            <dd>{{ timeAgo(selected.createdAt, now) }}</dd>
            <dt class="text-muted">
              Last seen
            </dt>
            <dd>{{ isOnline(selected) ? 'Online now' : lastSeen(selected) ?? 'Not recorded yet' }}</dd>
            <dt class="text-muted">
              Total actions
            </dt>
            <dd>{{ activity.count.toLocaleString() }}</dd>
          </dl>
        </FormSection>

        <FormSection title="Recent activity">
          <template #actions>
            <UButton
              size="md"
              color="neutral"
              variant="subtle"
              :to="{ path: '/activity', query: { viewer: String(selected.id) } }"
            >
              See all
            </UButton>
          </template>
          <div
            v-if="activity.recent.length"
            class="grid grid-cols-1 divide-y divide-(--ui-border)"
          >
            <div
              v-for="command in activity.recent"
              :key="command.id"
              class="flex min-h-13 items-center gap-3 py-1.5"
            >
              <b class="min-w-0 flex-1 truncate font-medium text-highlighted">{{ command.controlName }}</b>
              <span class="max-w-[45%] truncate rounded-full bg-secondary/15 px-2.5 py-0.5 text-xs text-secondary">{{ formatCommandValue(command.value) }}</span>
              <span class="shrink-0 font-mono text-xs text-muted">{{ timeAgo(command.createdAt) }}</span>
            </div>
          </div>
          <p
            v-else
            class="text-sm text-muted"
          >
            No recent activity from this viewer.
          </p>
        </FormSection>
      </div>
    </div>

    <EmptyState
      v-else
      icon="i-lucide-users"
      title="No viewers yet"
      description="People who open your share link show up here, with what they used and ban controls."
    />
  </PageLayout>
</template>
