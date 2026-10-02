<script setup lang="ts">
import FormSection from '@renderer/components/ui/FormSection.vue'
import { useAreYouSureModal } from '@renderer/composables/useAreYouSureModal'
import { useBannedClientsDb } from '@renderer/composables/useBannedClientsDb'
import { LOG_RETENTION_CHOICES, logRetentionDays, purgeOldCommands } from '@renderer/composables/useCommandsDb'
import { useLiveQuery } from '@renderer/composables/useLiveQuery'
import type { BannedClient, Client } from '@renderer/db/clients.db'
import { db as clientsDb } from '@renderer/db/clients.db'
import { db } from '@renderer/db/commands.db'

// Local data this app keeps: the activity log (and how long), known viewers, and bans - which
// used to be reachable only one viewer at a time from the clients list.
const toast = useToast()
const { bannedClients, unban } = useBannedClientsDb()
const { openModal: confirm } = useAreYouSureModal()

const counts = useLiveQuery(
  () => null,
  () => Promise.all([db.commands.count(), clientsDb.clients.count()]).then(([commands, clients]) => ({ commands, clients })),
  { commands: 0, clients: 0 }
)

const clients = useLiveQuery(
  () => null,
  () => clientsDb.clients.toArray(),
  [] as Client[]
)

// A ban targets an IP or a Discord id; show the viewer it matches when one is known.
const banLabel = (ban: BannedClient): string =>
  clients.value.find((client) => (ban.scope === 'ip' ? client.ip === ban.value : client.discordId === ban.value))?.displayName ??
  (ban.scope === 'ip' ? 'Unknown viewer (IP)' : 'Unknown viewer (Discord)')

const setRetention = (days: number): void => {
  logRetentionDays.value = days
  // Apply a shorter window right away instead of at the next scheduled purge.
  void purgeOldCommands()
}

const clearLog = async (): Promise<void> => {
  const ok = await confirm({
    title: 'Clear activity log?',
    message: `Deletes all **${counts.value.commands.toLocaleString()}** logged actions. Viewers and bans are kept.`,
    confirmText: 'Clear log'
  })

  if (!ok) return

  await db.commands.clear()
  toast.add({ title: 'Activity log cleared', icon: 'i-lucide-trash-2' })
}

const doUnban = async (ban: BannedClient): Promise<void> => {
  if (ban.id === undefined) return

  const ok = await confirm({ title: `Unban ${banLabel(ban)}?`, message: 'They can use your controls again.', confirmText: 'Unban', danger: false })

  if (ok) await unban(ban.id)
}

const formatDate = (timestamp: number): string => new Date(timestamp).toLocaleDateString(undefined, { day: 'numeric', month: 'short', year: 'numeric' })
</script>

<template>
  <FormSection title="Activity log">
    <UFormField
      label="Keep activity for"
      description="Older actions are deleted automatically."
    >
      <div class="flex flex-wrap gap-2">
        <UButton
          v-for="days in LOG_RETENTION_CHOICES"
          :key="days"
          size="md"
          class="rounded-full"
          :color="logRetentionDays === days ? 'primary' : 'neutral'"
          :variant="logRetentionDays === days ? 'soft' : 'subtle'"
          @click="setRetention(days)"
        >
          {{ days }} {{ days === 1 ? 'day' : 'days' }}
        </UButton>
      </div>
    </UFormField>
    <dl class="grid w-fit grid-cols-[auto_auto] gap-x-6 gap-y-1.5 text-[13.5px]">
      <dt class="text-muted">
        Actions logged
      </dt>
      <dd class="tabular-nums">
        {{ counts.commands.toLocaleString() }}
      </dd>
      <dt class="text-muted">
        Viewers known
      </dt>
      <dd class="tabular-nums">
        {{ counts.clients.toLocaleString() }}
      </dd>
    </dl>
    <UButton
      icon="i-lucide-trash-2"
      color="error"
      variant="soft"
      class="w-fit"
      :disabled="!counts.commands"
      @click="clearLog"
    >
      Clear activity log
    </UButton>
  </FormSection>

  <FormSection
    title="Banned viewers"
    :description="`${bannedClients.length} banned`"
  >
    <div class="grid divide-y divide-(--ui-border)">
      <div
        v-for="ban in bannedClients"
        :key="ban.id"
        class="flex min-h-14.5 items-center gap-3 py-1.5"
      >
        <span class="grid size-10 shrink-0 place-items-center rounded-md bg-error/12 text-error">
          <UIcon
            :name="ban.scope === 'ip' ? 'i-lucide-globe' : 'ic:baseline-discord'"
            class="size-5"
          />
        </span>
        <div class="grid min-w-0 flex-1">
          <b class="truncate font-medium text-highlighted">{{ banLabel(ban) }}</b>
          <small class="truncate text-xs text-muted">By {{ ban.scope === 'ip' ? 'IP' : 'Discord' }} · {{ formatDate(ban.createdAt) }}{{ ban.reason ? ` · ${ban.reason}` : '' }}</small>
        </div>
        <UButton
          size="md"
          color="secondary"
          variant="soft"
          @click="doUnban(ban)"
        >
          Unban
        </UButton>
      </div>
      <p
        v-if="!bannedClients.length"
        class="py-1 text-sm text-muted"
      >
        Nobody is banned.
      </p>
    </div>
  </FormSection>
</template>
