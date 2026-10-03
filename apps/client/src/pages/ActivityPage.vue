<script setup lang="ts">
import EmptyState from '@renderer/components/ui/EmptyState.vue'
import { logRetentionDays } from '@renderer/composables/useCommandsDb'
import { useControls } from '@renderer/composables/useControls'
import { useLiveQuery } from '@renderer/composables/useLiveQuery'
import type { Client } from '@renderer/db/clients.db'
import { db as clientsDb } from '@renderer/db/clients.db'
import type { Command } from '@renderer/db/commands.db'
import { db } from '@renderer/db/commands.db'
import { formatCommandValue, isOpenShockCommandValue } from '@renderer/utils/commandLog'
import type { ControlTypes } from '@toolkitosc/shared-ui'
import { CONTROL_TYPE_LABELS } from '@toolkitosc/shared-ui'
import { computed } from 'vue'
import { useRoute, useRouter } from 'vue-router'

// Every command viewers sent, newest first, in one log with filters for time, viewer, control
// and type - replacing the per-control and per-viewer log slideovers. A tile's Activity button and
// a viewer's "See all" open it pre-filtered through the query string.
type Range = 'hour' | 'day' | 'week' | 'all'

const RANGES: { value: Range; label: string }[] = [
  { value: 'hour', label: 'Last hour' },
  { value: 'day', label: 'Last 24h' },
  { value: 'week', label: '7 days' },
  { value: 'all', label: 'All' }
]

// Enough for a busy session without making the page sluggish; older rows are one filter away.
const ROW_LIMIT = 1000

const route = useRoute()
const router = useRouter()
const { controls } = useControls()

const query = computed(() => ({
  range: (RANGES.some((r) => r.value === route.query.range) ? route.query.range : 'day') as Range,
  viewer: typeof route.query.viewer === 'string' ? Number(route.query.viewer) : null,
  control: typeof route.query.control === 'string' ? route.query.control : null,
  type: typeof route.query.type === 'string' ? (route.query.type as ControlTypes) : null
}))

const setFilter = (key: 'range' | 'viewer' | 'control' | 'type', value: string | number | null): void => {
  const next = { ...route.query, [key]: value === null ? undefined : String(value) }

  void router.replace({ query: next })
}

const clients = useLiveQuery(
  () => null,
  () => clientsDb.clients.toArray(),
  [] as Client[]
)

const viewer = computed(() => clients.value.find((client) => client.id === query.value.viewer) ?? null)

const since = (range: Range): number => {
  const now = Date.now()

  if (range === 'hour') return now - 60 * 60 * 1000
  if (range === 'day') return now - 24 * 60 * 60 * 1000
  if (range === 'week') return now - 7 * 24 * 60 * 60 * 1000

  return 0
}

// Uses the narrowest index available (control, then viewer), then filters the rest in memory.
const commands = useLiveQuery(
  () => ({ ...query.value, viewer: viewer.value }),
  ({ range, control, viewer: client, type }) => {
    const from = since(range)
    const collection = control
      ? db.commands.where('[controlId+createdAt]').between([control, from], [control, Infinity])
      : client?.discordId
        ? db.commands.where('[discordId+createdAt]').between([client.discordId, from], [client.discordId, Infinity])
        : client
          ? db.commands.where('[ip+createdAt]').between([client.ip, from], [client.ip, Infinity])
          : db.commands.where('createdAt').aboveOrEqual(from)

    return collection
      .reverse()
      .filter((command) => (!type || command.type === type) && (!client || !control || (command.discordId ?? null) === client.discordId || command.ip === client.ip))
      .limit(ROW_LIMIT)
      .toArray()
  },
  [] as Command[]
)

const clientFor = (command: Command): Client | undefined =>
  clients.value.find((client) => client.ip === command.ip && client.discordId === (command.discordId || null))

const controlName = computed(() => {
  if (!query.value.control) return null

  for (const group of controls.value) {
    const control = group.controls.find((c) => c.id === query.value.control)

    if (control) return control.name
  }

  return 'Deleted control'
})

const TYPES: ControlTypes[] = ['boolean', 'slider', 'enum', 'open-shock-shocker', 'intiface-toy', 'preset']

// OpenShock rows name the shockers that actually fired rather than the control.
const title = (command: Command): string =>
  isOpenShockCommandValue(command.value) && command.value.shockers.length
    ? command.value.shockers.join(', ')
    : command.controlName || command.controlId

const time = (timestamp: number): string =>
  new Date(timestamp).toLocaleString([], { hour: '2-digit', minute: '2-digit', second: '2-digit', ...(Date.now() - timestamp > 86_400_000 ? { day: 'numeric', month: 'short' } : {}) })

const COLUMNS = 'grid-cols-[6.5rem_minmax(0,11rem)_minmax(0,1fr)_7.5rem_minmax(0,9rem)]'
</script>

<template>
  <PageLayout>
    <template #sidebar>
      <div class="flex min-h-9 shrink-0 items-center">
        <b class="text-xl font-semibold text-highlighted">Activity</b>
      </div>

      <div class="-mr-1.5 flex min-h-0 flex-1 flex-col gap-4 overflow-y-auto pr-1.5">
        <div
          v-if="query.control"
          class="flex items-center gap-2.5 rounded-field glass px-3 py-2.5 text-sm"
        >
          <UIcon
            name="i-lucide-filter"
            class="size-4.5 shrink-0 text-primary"
          />
          <div class="grid min-w-0 flex-1">
            <span class="text-xs text-muted">Control</span>
            <b class="truncate font-medium text-highlighted">{{ controlName }}</b>
          </div>
          <UButton
            icon="i-lucide-x"
            size="sm"
            color="neutral"
            variant="ghost"
            aria-label="Clear control filter"
            @click="setFilter('control', null)"
          />
        </div>

        <UFormField label="Time">
          <div class="flex flex-wrap gap-2">
            <UButton
              v-for="range in RANGES"
              :key="range.value"
              size="md"
              :color="query.range === range.value ? 'primary' : 'neutral'"
              :variant="query.range === range.value ? 'soft' : 'subtle'"
              class="rounded-full"
              @click="setFilter('range', range.value === 'day' ? null : range.value)"
            >
              {{ range.label }}
            </UButton>
          </div>
        </UFormField>

        <UFormField label="Viewer">
          <div class="flex flex-wrap gap-2">
            <UButton
              size="md"
              :color="!viewer ? 'primary' : 'neutral'"
              :variant="!viewer ? 'soft' : 'subtle'"
              class="rounded-full"
              @click="setFilter('viewer', null)"
            >
              Everyone
            </UButton>
            <UButton
              v-for="client in clients.slice(0, 12)"
              :key="client.id"
              size="md"
              :color="viewer?.id === client.id ? 'primary' : 'neutral'"
              :variant="viewer?.id === client.id ? 'soft' : 'subtle'"
              class="max-w-40 rounded-full"
              :avatar="{ src: client.avatar, alt: client.displayName }"
              @click="setFilter('viewer', client.id ?? null)"
            >
              <span class="truncate">{{ client.displayName }}</span>
            </UButton>
          </div>
        </UFormField>

        <UFormField label="Type">
          <div class="flex flex-wrap gap-2">
            <UButton
              size="md"
              :color="!query.type ? 'primary' : 'neutral'"
              :variant="!query.type ? 'soft' : 'subtle'"
              class="rounded-full"
              @click="setFilter('type', null)"
            >
              All
            </UButton>
            <UButton
              v-for="type in TYPES"
              :key="type"
              size="md"
              :color="query.type === type ? 'primary' : 'neutral'"
              :variant="query.type === type ? 'soft' : 'subtle'"
              class="rounded-full"
              @click="setFilter('type', type)"
            >
              {{ CONTROL_TYPE_LABELS[type] }}
            </UButton>
          </div>
        </UFormField>
      </div>

      <p class="flex shrink-0 items-center gap-1.5 px-1 text-xs text-muted">
        <UIcon
          name="i-lucide-clock"
          class="size-3.5"
        />
        Activity is kept for {{ logRetentionDays }} {{ logRetentionDays === 1 ? 'day' : 'days' }}.
      </p>
    </template>

    <template #header>
      <PageHeader
        :title="controlName ? `Activity · ${controlName}` : viewer ? `Activity · ${viewer.displayName}` : 'Activity'"
        :meta="`${commands.length === ROW_LIMIT ? `Latest ${ROW_LIMIT}` : commands.length} actions`"
      />
    </template>

    <div
      v-if="commands.length"
      class="flex h-full min-h-80 flex-col overflow-hidden rounded-field border border-default bg-(--aurora-glass)"
    >
      <div
        class="grid shrink-0 items-center gap-3 border-b border-default px-3.5 py-2.5 font-mono text-[11px] tracking-widest text-muted uppercase"
        :class="COLUMNS"
      >
        <span>Time</span>
        <span>Viewer</span>
        <span>Control</span>
        <span>Type</span>
        <span>Value</span>
      </div>
      <!-- Virtualized: a busy session logs thousands of commands. -->
      <UScrollArea
        :items="commands"
        virtualize
        class="min-h-0 flex-1"
      >
        <template #default="{ item: command }">
          <div
            class="grid min-h-14.5 items-center gap-3 border-b border-default px-3.5 py-2"
            :class="COLUMNS"
          >
            <span class="font-mono text-xs text-muted tabular-nums">{{ time(command.createdAt) }}</span>
            <RouterLink
              v-if="clientFor(command)"
              :to="`/viewers/${clientFor(command)!.id}`"
              class="flex min-w-0 items-center gap-2 hover:underline"
            >
              <UAvatar
                :src="clientFor(command)!.avatar"
                :alt="clientFor(command)!.displayName"
                size="xs"
              />
              <span class="truncate text-sm">{{ clientFor(command)!.displayName }}</span>
            </RouterLink>
            <span
              v-else
              class="text-sm text-muted"
            >Unknown</span>
            <b class="truncate font-medium text-highlighted">{{ title(command) }}</b>
            <span class="w-fit truncate rounded-full bg-(--aurora-well) px-2 py-0.5 text-xs text-muted">{{ CONTROL_TYPE_LABELS[command.type as ControlTypes] ?? command.type }}</span>
            <span class="w-fit max-w-full truncate rounded-full bg-secondary/15 px-2.5 py-0.5 text-xs text-secondary">{{ formatCommandValue(command.value) }}</span>
          </div>
        </template>
      </UScrollArea>
    </div>

    <EmptyState
      v-else
      icon="i-lucide-history"
      title="No activity"
      :description="query.range === 'all' && !query.control && !viewer && !query.type ? 'Nothing has been logged yet. Commands from viewers show up here.' : 'Nothing matches these filters.'"
    />
  </PageLayout>
</template>
