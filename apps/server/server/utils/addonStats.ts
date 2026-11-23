import { count, eq, gte, sql } from 'drizzle-orm'

// Relative, not ~~: test/addonStats.test.ts imports this file outside Nuxt.
import { getDb } from '../db'
import { addonUsage, users } from '../db/schema'

export interface AddonUsageEntry {
  addonId: string
  enabled: boolean
  devices: number | null
}

const ADDON_ID = /^[a-z0-9-]{1,32}$/
// Well above the add-ons the app has; anything past it is junk, not a real app.
const MAX_ADDONS = 32
const MAX_DEVICES = 999

// The host's `addon-stats` is untrusted: keeps well-formed entries only, one per id.
export function parseAddonStats(message: unknown): AddonUsageEntry[] {
  const list = (message as { addons?: unknown } | null)?.addons

  if (!Array.isArray(list)) return []

  const entries = new Map<string, AddonUsageEntry>()

  for (const item of list.slice(0, MAX_ADDONS)) {
    const { id, enabled, devices } = (item ?? {}) as Record<string, unknown>

    if (typeof id !== 'string' || !ADDON_ID.test(id) || typeof enabled !== 'boolean') continue

    const count = enabled && typeof devices === 'number' && Number.isInteger(devices) && devices >= 0 ? Math.min(devices, MAX_DEVICES) : null

    entries.set(id, { addonId: id, enabled, devices: count })
  }

  return [...entries.values()]
}

// Snapshot replace, like recordControlInventory: an add-on missing from the message disappears.
export async function recordAddonUsage(discordId: string, entries: AddonUsageEntry[]): Promise<void> {
  const db = getDb()
  const clear = db.delete(addonUsage).where(eq(addonUsage.discordId, discordId))

  if (!entries.length) {
    await clear
    return
  }

  const updatedAt = new Date()

  await db.batch([clear, db.insert(addonUsage).values(entries.map((entry) => ({ discordId, ...entry, updatedAt })))])
}

// One account's add-ons, for its page on the dashboard.
export async function userAddonUsage(discordId: string): Promise<AddonUsageEntry[]> {
  return getDb()
    .select({ addonId: addonUsage.addonId, enabled: addonUsage.enabled, devices: addonUsage.devices })
    .from(addonUsage)
    .where(eq(addonUsage.discordId, discordId))
}

// Every account's add-ons that are on, for the users table.
export async function enabledAddonsByUser(): Promise<Map<string, string[]>> {
  const rows = await getDb().select({ discordId: addonUsage.discordId, addonId: addonUsage.addonId }).from(addonUsage).where(eq(addonUsage.enabled, true))
  const byUser = new Map<string, string[]>()

  for (const row of rows) byUser.set(row.discordId, [...(byUser.get(row.discordId) ?? []), row.addonId])

  return byUser
}

// Per add-on, over the accounts seen in the last `days` days: how many could run it (reported it,
// on or off), how many have it on, and the devices those have.
export async function addonUsageTotals(days: number) {
  return getDb()
    .select({
      addonId: addonUsage.addonId,
      reported: count(),
      enabled: sql<number>`coalesce(sum(${addonUsage.enabled}), 0)`,
      devices: sql<number>`coalesce(sum(${addonUsage.devices}), 0)`,
      // Accounts with at least one device, so "average per account" isn't dragged down by setups
      // that never connected one.
      withDevices: sql<number>`coalesce(sum(${addonUsage.devices} > 0), 0)`
    })
    .from(addonUsage)
    .innerJoin(users, eq(users.discordId, addonUsage.discordId))
    .where(gte(users.clientSeenAt, new Date(Date.now() - days * 86_400_000)))
    .groupBy(addonUsage.addonId)
}
