import { viewersSetup } from '@renderer/composables/useSetup'
import type { Ban, BanScope } from '@renderer/lib/tauri-bridge'
import { api } from '@renderer/lib/tauri-bridge'
import { computed } from 'vue'

// Bans, kept by Rust (src-tauri/src/setup/viewers.rs), which checks every viewer command against
// them. `isBanned` stays synchronous for the Viewers page.
const bannedClients = computed<Ban[]>(() => viewersSetup.value.bans)

export function useBannedClientsDb(): {
  bannedClients: typeof bannedClients
  ban: (scope: BanScope, value: string, reason?: string) => Promise<void>
  unban: (ban: Ban) => Promise<void>
  isBanned: (ip: string, discordId?: string | null) => Ban | undefined
} {
  const ban = (scope: BanScope, value: string, reason?: string): Promise<void> => api.setupBan(scope, value, reason)

  const unban = (ban: Ban): Promise<void> => api.setupUnban(ban.scope, ban.value)

  const isBanned = (ip: string, discordId?: string | null): Ban | undefined =>
    bannedClients.value.find(
      (banned) =>
        (banned.scope === 'ip' && banned.value === ip) ||
        (banned.scope === 'discord' && !!discordId && banned.value === discordId)
    )

  return { bannedClients, ban, unban, isBanned }
}
