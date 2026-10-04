<script setup lang="ts">
import type { ContextMenuItem } from '@nuxt/ui/components/ContextMenu.vue'
import EmptyState from '@renderer/components/ui/EmptyState.vue'
import FormSection from '@renderer/components/ui/FormSection.vue'
import { useAreYouSureModal } from '@renderer/composables/useAreYouSureModal'
import { useBannedClientsDb } from '@renderer/composables/useBannedClientsDb'
import { useBanViewerModal } from '@renderer/composables/useBanViewerModal'
import { useClientsDb } from '@renderer/composables/useClientsDb'
import { useLiveQuery } from '@renderer/composables/useLiveQuery'
import type { Client } from '@renderer/db/clients.db'
import { db as clientsDb } from '@renderer/db/clients.db'
import type { Command } from '@renderer/db/commands.db'
import { timeAgo } from '@renderer/utils/time'
import { countViewerCommands, viewerCommands } from '@renderer/utils/viewers'
import { formatCommandValue } from '@toolkitosc/shared-ui'
import { useNow } from '@vueuse/core'
import { computed, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'

// Everyone who has ever connected to your share page: online, offline and banned in the sidebar;
// the selected viewer's details, ban state (with its reason, now shown) and recent activity in the
// main pane. Replaces the clients slideover and per-client log modal. Every viewer action is in one
// menu: right-click a viewer in the list, or the ⋯ button in the header (for VR, where there's no
// right-click).
const RECENT_LIMIT = 10

type Kind = 'discord' | 'guest'

const KINDS: { value: Kind | null; label: string }[] = [
  { value: null, label: 'All' },
  { value: 'discord', label: 'Discord' },
  { value: 'guest', label: 'Guests' }
]

const route = useRoute()
const router = useRouter()
const toast = useToast()
const { isOnline, clearViewerActivity, deleteViewer } = useClientsDb()
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

// In the query string, like the Activity page's filters, so it survives leaving the page.
const kind = computed<Kind | null>(() => (route.query.type === 'discord' || route.query.type === 'guest' ? route.query.type : null))

const setKind = (value: Kind | null): void => void router.replace({ query: { ...route.query, type: value ?? undefined } })

const kindOf = (client: Client): Kind => (client.discordId ? 'discord' : 'guest')

const kindCount = (value: Kind | null): number => (value ? clients.value.filter((client) => kindOf(client) === value).length : clients.value.length)

const filtered = computed(() => {
  const query = search.value.trim().toLowerCase()

  return clients.value
    .filter((client) => !kind.value || kindOf(client) === kind.value)
    .filter((client) => !query || client.displayName.toLowerCase().includes(query) || client.discordId?.includes(query))
    .sort((a, b) => a.displayName.localeCompare(b.displayName))
})

// Offline viewers most recently seen first; those never stamped fall back to when they were first seen.
const seenAt = (client: Client): number => client.lastSeenAt ?? client.createdAt

const sections = computed(() => [
  { key: 'online', label: 'Online', clients: filtered.value.filter((c) => !isBanned(c.ip, c.discordId) && isOnline(c)) },
  {
    key: 'offline',
    label: 'Offline',
    clients: filtered.value.filter((c) => !isBanned(c.ip, c.discordId) && !isOnline(c)).sort((a, b) => seenAt(b) - seenAt(a))
  },
  { key: 'banned', label: 'Banned', clients: filtered.value.filter((c) => isBanned(c.ip, c.discordId)) }
])

const selectedId = computed(() => Number(route.params.clientId) || null)
const selected = computed(() => clients.value.find((client) => client.id === selectedId.value) ?? null)

// /viewers alone opens the first viewer (online ones first). One that a filter hides stays open.
watch(
  [sections, selectedId],
  () => {
    if (selected.value || !clients.value.length) return
    const first = sections.value.flatMap((section) => section.clients)[0]
    if (first?.id !== undefined) void router.replace({ path: `/viewers/${first.id}`, query: route.query })
  },
  { immediate: true }
)

const banned = computed(() => (selected.value ? isBanned(selected.value.ip, selected.value.discordId) : undefined))

const commandsFor = (client: Client | null): Promise<{ count: number; recent: Command[] }> => {
  if (!client) return Promise.resolve({ count: 0, recent: [] })

  return Promise.all([countViewerCommands(client), viewerCommands(client, { limit: RECENT_LIMIT })]).then(([count, recent]) => ({ count, recent }))
}

const activity = useLiveQuery(selected, commandsFor, { count: 0, recent: [] as Command[] })

const showIp = ref(false)

watch(selectedId, () => (showIp.value = false))

// Most recent first. Hidden by default, since the page may be on stream.
const maskedIps = computed(() =>
  (selected.value?.ips ?? []).map((ip) => (showIp.value ? ip : ip.replace(/[^.:]+(?=[.:][^.:]+$)|[^.:]+$/g, '•••')))
)

// Actions take the viewer they're for rather than reading `selected`: right-clicking a viewer in
// the list doesn't open them.
const doBan = async (client: Client): Promise<void> => {
  const result = await openBanModal(client)

  if (!result) return

  await ban(result.scope, result.scope === 'discord' ? client.discordId! : client.ip, result.reason)
  toast.add({ title: `${client.displayName} banned`, icon: 'i-lucide-ban' })
}

const doUnban = async (client: Client): Promise<void> => {
  const entry = isBanned(client.ip, client.discordId)

  if (entry?.id === undefined) return

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

const actionCount = (count: number): string => `**${count.toLocaleString()}** logged ${count === 1 ? 'action' : 'actions'}`

const doClearActivity = async (client: Client): Promise<void> => {
  const count = await countViewerCommands(client)

  if (!count) {
    toast.add({ title: `${client.displayName} has no activity to clear`, icon: 'i-lucide-info' })
    return
  }

  const ok = await confirm({
    title: `Clear ${client.displayName}'s activity?`,
    message: `Deletes their ${actionCount(count)}. They stay in your viewers.`,
    confirmText: 'Clear activity'
  })

  if (!ok) return

  await clearViewerActivity(client)
  toast.add({ title: `Cleared ${client.displayName}'s activity`, icon: 'i-lucide-eraser' })
}

const doDelete = async (client: Client): Promise<void> => {
  const count = await countViewerCommands(client)
  // The server lists someone who's still connected again on its next update, as a new viewer.
  const online = isOnline(client) ? " They're online now, so they'll show up again as a new viewer." : ''

  const ok = await confirm({
    title: `Delete ${client.displayName}?`,
    message: `Removes them and their ${actionCount(count)}. Bans stay in place.${online}`,
    confirmText: 'Delete'
  })

  if (!ok) return

  await deleteViewer(client)
  toast.add({ title: `${client.displayName} deleted`, icon: 'i-lucide-trash-2' })
  if (client.id === selectedId.value) void router.replace({ path: '/viewers', query: route.query })
}

const copyDiscordId = async (discordId: string): Promise<void> => {
  try {
    await navigator.clipboard.writeText(discordId)
    toast.add({ title: 'Discord ID copied', icon: 'i-lucide-check', color: 'success' })
  } catch {
    toast.add({ title: 'Could not copy Discord ID', color: 'error' })
  }
}

const viewerMenu = (client: Client): ContextMenuItem[][] => [
  [
    { label: 'See activity', icon: 'i-lucide-history', to: { path: '/activity', query: { viewer: String(client.id) } } },
    ...(client.discordId ? [{ label: 'Copy Discord ID', icon: 'i-lucide-copy', onSelect: () => copyDiscordId(client.discordId!) }] : [])
  ],
  [
    isBanned(client.ip, client.discordId)
      ? { label: 'Unban…', icon: 'i-lucide-shield-check', onSelect: () => doUnban(client) }
      : { label: 'Ban…', icon: 'i-lucide-ban', onSelect: () => doBan(client) }
  ],
  [
    { label: 'Clear activity…', icon: 'i-lucide-eraser', color: 'error', onSelect: () => doClearActivity(client) },
    { label: 'Delete viewer…', icon: 'i-lucide-trash-2', color: 'error', onSelect: () => doDelete(client) }
  ]
]

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
      <div class="flex shrink-0 flex-wrap gap-2">
        <UButton
          v-for="option in KINDS"
          :key="option.label"
          size="md"
          :color="kind === option.value ? 'primary' : 'neutral'"
          :variant="kind === option.value ? 'soft' : 'subtle'"
          class="rounded-full"
          @click="setKind(option.value)"
        >
          {{ option.label }}
          <span class="font-mono text-xs opacity-70">{{ kindCount(option.value) }}</span>
        </UButton>
      </div>

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
          <UContextMenu
            v-for="client in section.clients"
            :key="client.id"
            :items="viewerMenu(client)"
          >
            <RouterLink
              :to="{ path: `/viewers/${client.id}`, query: route.query }"
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
                {{ client.discordId ? 'Discord' : 'Guest' }}<template v-if="section.key !== 'online' && lastSeen(client)"> · last seen {{ lastSeen(client) }}</template>
              </small>
            </RouterLink>
          </UContextMenu>
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
            >{{ selected.discordId ? 'Discord account' : 'Guest' }}</span>
          </div>
        </div>
      </div>
      <UButton
        v-if="banned"
        icon="i-lucide-shield-check"
        color="secondary"
        variant="soft"
        @click="doUnban(selected)"
      >
        Unban
      </UButton>
      <UButton
        v-else
        icon="i-lucide-ban"
        color="error"
        variant="soft"
        @click="doBan(selected)"
      >
        Ban
      </UButton>
      <UDropdownMenu
        :items="viewerMenu(selected)"
        size="xl"
        :content="{ align: 'end' }"
      >
        <UButton
          icon="i-lucide-ellipsis"
          color="neutral"
          variant="subtle"
          :aria-label="`${selected.displayName} actions`"
        />
      </UDropdownMenu>
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
            <dt class="self-start pt-1 text-muted">
              {{ maskedIps.length > 1 ? `IP addresses (${maskedIps.length})` : 'IP address' }}
            </dt>
            <dd class="flex min-w-0 items-start gap-2 font-mono">
              <span class="grid min-w-0 gap-0.5 pt-1">
                <span
                  v-for="(ip, index) in maskedIps"
                  :key="index"
                  class="truncate"
                  :class="{ 'text-muted': index > 0 }"
                >{{ ip }}</span>
              </span>
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
