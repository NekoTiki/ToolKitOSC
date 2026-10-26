import { z } from 'zod'

import { getDb } from '~~/server/db'
import { serverSettings } from '~~/server/db/schema'
import { MAX_SOCKETS_PER_IP, MAX_VIEWERS_PER_ROOM } from '~~/server/utils/connectionCaps'

// What admins can change from the dashboard's Settings page without a redeploy. Read synchronously
// (share pages check the caps on every join), from a copy loaded at startup (plugins/settings.ts)
// and replaced on every save. The admin list stays in NUXT_AI_ADMIN_DISCORD_IDS on purpose.
export const settingsSchema = z.object({
  // Off: nobody gets AI suggestions, admins included.
  aiEnabled: z.boolean(),
  dailyCredits: z.number().int().min(0).max(10_000),
  // '' spreads accounts across every configured provider.
  defaultProvider: z.string().max(32),
  defaultProfile: z.enum(['light', 'balanced', 'heavy']),
  maxViewersPerRoom: z.number().int().min(1).max(100_000),
  maxSocketsPerIp: z.number().int().min(1).max(1000),
  retentionDays: z.number().int().min(7).max(3650)
})

export type ServerSettings = z.infer<typeof settingsSchema>

const defaults = (): ServerSettings => {
  const config = useRuntimeConfig()

  return {
    aiEnabled: true,
    dailyCredits: 20,
    defaultProvider: config.aiDefaultProvider,
    defaultProfile: settingsSchema.shape.defaultProfile.safeParse(config.aiDefaultProfile).data ?? 'balanced',
    maxViewersPerRoom: MAX_VIEWERS_PER_ROOM,
    maxSocketsPerIp: MAX_SOCKETS_PER_IP,
    retentionDays: 90
  }
}

let current: ServerSettings | null = null

export function getSettings(): ServerSettings {
  return current ?? defaults()
}

// Saved values over the defaults. A value an older or newer server saved that no longer fits is
// dropped, rather than breaking startup.
export async function loadSettings(): Promise<void> {
  const rows = await getDb().select().from(serverSettings)
  const loaded: Record<string, unknown> = defaults()

  for (const row of rows) {
    const field = settingsSchema.shape[row.key as keyof ServerSettings]
    const parsed = field?.safeParse(row.value)

    if (parsed?.success) loaded[row.key] = parsed.data
  }

  current = loaded as ServerSettings
}

export async function saveSettings(changes: Partial<ServerSettings>): Promise<ServerSettings> {
  for (const [key, value] of Object.entries(changes)) {
    if (value === undefined) continue

    await getDb().insert(serverSettings).values({ key, value }).onConflictDoUpdate({ target: serverSettings.key, set: { value } })
  }

  await loadSettings()

  return getSettings()
}
