import { and, eq, gte, sql } from 'drizzle-orm'

import type { ControlGroup, ControlTypes } from '#shared/types/protocol'
import { getDb } from '~~/server/db'
import { controlActivationDaily, controlInventory } from '~~/server/db/schema'

// UTC calendar day, matching rateLimit.ts's own `today()` - a new day starts every counter at zero
// with no explicit reset write needed.
function today(): string {
  return new Date().toISOString().slice(0, 10)
}

// Snapshot replace: wholesale, but scoped to just this (discordId, avatarId) pair - matching how
// the in-memory `hostControls` map itself is replaced on every `controls-update` (see
// routes/host.ts), while leaving every *other* avatar's own last-recorded snapshot untouched. A
// type dropped from this avatar's payload just disappears from its slice, rather than needing to
// be explicitly zeroed out. `avatarId` is null only in the brief window before the host has loaded
// any avatar (see ControlsUpdateMessage's own comment) - there's nothing to key a snapshot by yet,
// so it's skipped entirely rather than recorded under some placeholder id.
export async function recordControlInventory(discordId: string, avatarId: string | null, groups: ControlGroup[]): Promise<void> {
  if (!avatarId) return

  const counts = new Map<ControlTypes, number>()

  for (const group of groups) {
    for (const control of group.controls) {
      counts.set(control.type, (counts.get(control.type) ?? 0) + 1)
    }
  }

  const updatedAt = new Date()
  const db = getDb()
  const clear = db.delete(controlInventory).where(and(eq(controlInventory.discordId, discordId), eq(controlInventory.avatarId, avatarId)))

  // A batch, not db.transaction(): libsql hands its connection to an interactive transaction and
  // opens a fresh one (without a busy timeout) for everything else, so two overlapping
  // controls-updates failed with SQLITE_BUSY. A batch runs atomically on the one shared connection.
  if (counts.size) {
    await db.batch([
      clear,
      db.insert(controlInventory).values(Array.from(counts.entries()).map(([type, count]) => ({ discordId, avatarId, type, count, updatedAt })))
    ])
  } else {
    await clear
  }
}

// One row per (discordId, type, day), incremented on every control activation relayed to that
// host's room (see routes/ws/[ws].ts). Read-then-write, same as rateLimit.ts's credit counters -
// fine at this traffic level (a single host's own control panel), not a real concurrency concern.
async function recordControlActivation(discordId: string, type: ControlTypes): Promise<void> {
  const day = today()
  const db = getDb()

  const [existing] = await db
    .select({ count: controlActivationDaily.count })
    .from(controlActivationDaily)
    .where(
      and(
        eq(controlActivationDaily.discordId, discordId),
        eq(controlActivationDaily.type, type),
        eq(controlActivationDaily.day, day)
      )
    )
    .limit(1)

  const newCount = (existing?.count ?? 0) + 1

  await db
    .insert(controlActivationDaily)
    .values({ discordId, type, day, count: newCount })
    .onConflictDoUpdate({
      target: [controlActivationDaily.discordId, controlActivationDaily.type, controlActivationDaily.day],
      set: { count: newCount }
    })
}

function logActivationFailure(error: unknown): void {
  console.error('[controls] failed to record activation:', error)
}

// Continuous/drag-based controls (see ControlSliderBase.vue, shared by 'slider' and 'intiface-toy')
// send a burst of commands while being dragged - every intermediate value along the drag, not one
// command per user interaction the way a click/toggle sends. Counting each of those as its own
// activation would wildly overcount a single drag versus every other control type.
const DEBOUNCED_ACTIVATION_TYPES = new Set<ControlTypes>(['slider', 'intiface-toy'])

// The wire protocol has no explicit "drag released" message (ControlSliderBase.vue's mouseup
// handler never reaches the server, see its own comment) - this approximates it instead: reset a
// per-control timer on every command, and only actually count the activation once that control has
// gone quiet for this long, i.e. once the drag has settled. Comfortably longer than the slider's
// own commit debounce (50ms default, see ControlSliderBase.vue) so an active drag never gets
// counted mid-motion, short enough that a real release still shows up as "just now".
const RELEASE_DEBOUNCE_MS = 500

const pendingReleaseActivations = new Map<string, ReturnType<typeof setTimeout>>()

// Entry point for every relayed control command (see routes/ws/[ws].ts) - decides whether to count
// it immediately (a discrete control) or debounce it toward a single "release" count (a continuous
// one, per DEBOUNCED_ACTIVATION_TYPES above). Fire-and-forget either way: this must never add DB
// latency to the relay itself, most of all a dragged slider's own round trip.
export function scheduleControlActivation(discordId: string, groupId: string, controlId: string, type: ControlTypes): void {
  if (!DEBOUNCED_ACTIVATION_TYPES.has(type)) {
    void recordControlActivation(discordId, type).catch(logActivationFailure)

    return
  }

  const key = `${discordId}:${groupId}:${controlId}`

  clearTimeout(pendingReleaseActivations.get(key))

  pendingReleaseActivations.set(
    key,
    setTimeout(() => {
      pendingReleaseActivations.delete(key)
      void recordControlActivation(discordId, type).catch(logActivationFailure)
    }, RELEASE_DEBOUNCE_MS)
  )
}

export interface ControlStatsResult {
  inventory: { type: string; count: number }[]
  totalControls: number
  activationByType: { type: string; count: number }[]
  activationDaily: { day: string; count: number }[]
  totalActivations: number
}

function buildControlStatsResult(
  inventory: { type: string; count: number }[],
  activation: { day: string; type: string; count: number }[]
): ControlStatsResult {
  const activationByTypeMap = new Map<string, number>()
  const activationByDayMap = new Map<string, number>()

  for (const row of activation) {
    activationByTypeMap.set(row.type, (activationByTypeMap.get(row.type) ?? 0) + row.count)
    activationByDayMap.set(row.day, (activationByDayMap.get(row.day) ?? 0) + row.count)
  }

  return {
    inventory,
    totalControls: inventory.reduce((sum, r) => sum + r.count, 0),
    activationByType: Array.from(activationByTypeMap.entries()).map(([type, count]) => ({ type, count })),
    activationDaily: Array.from(activationByDayMap.entries())
      .map(([day, count]) => ({ day, count }))
      .sort((a, b) => a.day.localeCompare(b.day)),
    totalActivations: activation.reduce((sum, r) => sum + r.count, 0)
  }
}

// Summed across every avatar (optionally narrowed to one discordId) - there's one inventory row
// per (discordId, avatarId, type), so a plain per-type total has to add them back up across
// whichever avatars are in scope. See controlInventory's own schema comment for why avatarId is
// part of the key at all.
async function queryInventoryByType(discordId?: string): Promise<{ type: string; count: number }[]> {
  const rows = await getDb()
    .select({ type: controlInventory.type, count: sql<number>`sum(${controlInventory.count})` })
    .from(controlInventory)
    .where(discordId ? eq(controlInventory.discordId, discordId) : undefined)
    .groupBy(controlInventory.type)

  return rows.map((r) => ({ type: r.type, count: Number(r.count) }))
}

export async function queryUserControlStats(discordId: string, days: number): Promise<ControlStatsResult> {
  const db = getDb()
  const sinceDay = new Date(Date.now() - days * 24 * 60 * 60 * 1000).toISOString().slice(0, 10)

  const [inventory, activation] = await Promise.all([
    queryInventoryByType(discordId),
    db
      .select({
        day: controlActivationDaily.day,
        type: controlActivationDaily.type,
        count: controlActivationDaily.count
      })
      .from(controlActivationDaily)
      .where(and(eq(controlActivationDaily.discordId, discordId), gte(controlActivationDaily.day, sinceDay)))
  ])

  return buildControlStatsResult(inventory, activation)
}

// Same shape as queryUserControlStats, but summed across every host instead of filtered to one -
// backs the /dashboard/stats global "Controls" section the way queryGlobalControlStats' AI
// counterpart (queryStats in generationLog.ts) backs its existing charts.
export async function queryGlobalControlStats(days: number): Promise<ControlStatsResult> {
  const sinceDay = new Date(Date.now() - days * 24 * 60 * 60 * 1000).toISOString().slice(0, 10)

  const [inventory, activation] = await Promise.all([
    queryInventoryByType(),
    getDb()
      .select({
        day: controlActivationDaily.day,
        type: controlActivationDaily.type,
        count: sql<number>`sum(${controlActivationDaily.count})`
      })
      .from(controlActivationDaily)
      .where(gte(controlActivationDaily.day, sinceDay))
      .groupBy(controlActivationDaily.day, controlActivationDaily.type)
  ])

  return buildControlStatsResult(
    inventory,
    activation.map((r) => ({ day: r.day, type: r.type, count: Number(r.count) }))
  )
}
