import type { BannedClient, BanScope } from '@renderer/db/clients.db'
import { db } from '@renderer/db/clients.db'
import { liveQuery } from 'dexie'
import { ref } from 'vue'

const bannedClients = ref<BannedClient[]>([])

// Kept in sync with the DB at module scope (same pattern as `onlineClients` in useClientsDb.ts)
// so `isBanned` can be checked synchronously from the hot command path in useWebsocketHost.ts.
liveQuery(() => db.bannedClients.toArray()).subscribe({
  next: (value) => (bannedClients.value = value),
  error: (error) => console.error('Failed to load banned clients', error)
})

export function useBannedClientsDb(): {
  bannedClients: typeof bannedClients
  ban: (scope: BanScope, value: string, reason?: string) => Promise<void>
  unban: (id: number) => Promise<void>
  isBanned: (ip: string, discordId?: string | null) => BannedClient | undefined
} {
  const ban = async (scope: BanScope, value: string, reason?: string): Promise<void> => {
    const existing = await db.bannedClients.get({ scope, value })

    if (existing) {
      await db.bannedClients.put({ ...existing, reason: reason ?? existing.reason })
    } else {
      await db.bannedClients.add({ scope, value, reason, createdAt: Date.now() })
    }
  }

  const unban = async (id: number): Promise<void> => {
    await db.bannedClients.delete(id)
  }

  const isBanned = (ip: string, discordId?: string | null): BannedClient | undefined =>
    bannedClients.value.find(
      (banned) =>
        (banned.scope === 'ip' && banned.value === ip) ||
        (banned.scope === 'discord' && !!discordId && banned.value === discordId)
    )

  return { bannedClients, ban, unban, isBanned }
}
