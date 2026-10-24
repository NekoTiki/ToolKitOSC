import { eq, gte } from 'drizzle-orm'

import { getDb } from '~~/server/db'
import { relayDaily } from '~~/server/db/schema'

// How busy the relay is, per UTC day: the most hosts and viewers online at once, viewers joining,
// and share-page messages rate limited. Counted in memory and saved every minute (plugins/relayStats.ts).
interface Day {
  day: string
  peakHosts: number
  peakViewers: number
  joins: number
  rateLimited: number
}

const today = (): string => new Date().toISOString().slice(0, 10)
const empty = (): Day => ({ day: today(), peakHosts: 0, peakViewers: 0, joins: 0, rateLimited: 0 })

let current = empty()
let dirty = false

const save = async (row: Day): Promise<void> => {
  const { peakHosts, peakViewers, joins, rateLimited } = row

  await getDb().insert(relayDaily).values(row).onConflictDoUpdate({ target: relayDaily.day, set: { peakHosts, peakViewers, joins, rateLimited } })
}

// Today's numbers, marked for the next save. A new UTC day starts from zero, after saving the last one.
const day = (): Day => {
  if (current.day !== today()) {
    if (dirty) void save(current).catch((error) => console.error('[relay-stats] save failed:', error))
    current = empty()
  }

  dirty = true

  return current
}

export function noteHosts(online: number): void {
  const row = day()

  row.peakHosts = Math.max(row.peakHosts, online)
}

export function noteViewers(online: number): void {
  const row = day()

  row.peakViewers = Math.max(row.peakViewers, online)
}

export function noteJoin(): void {
  day().joins++
}

export function noteRateLimited(): void {
  day().rateLimited++
}

// Picks up today's numbers after a restart, so they keep adding up.
export async function loadRelayStats(): Promise<void> {
  const [row] = await getDb().select().from(relayDaily).where(eq(relayDaily.day, today())).limit(1)

  if (row) current = { ...row }
}

export async function flushRelayStats(): Promise<void> {
  if (!dirty) return

  dirty = false
  await save(current)
}

export async function queryRelayStats(days: number) {
  const since = new Date(Date.now() - days * 86_400_000).toISOString().slice(0, 10)

  await flushRelayStats()

  return getDb().select().from(relayDaily).where(gte(relayDaily.day, since)).orderBy(relayDaily.day)
}
