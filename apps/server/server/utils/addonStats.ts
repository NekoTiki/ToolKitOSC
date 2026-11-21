import { eq } from 'drizzle-orm'

// Relative, not ~~: test/addonStats.test.ts imports this file outside Nuxt.
import { getDb } from '../db'
import { addonUsage } from '../db/schema'

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
