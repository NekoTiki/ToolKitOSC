import { desc, eq } from 'drizzle-orm'

import { getDb } from '~~/server/db'
import { aiAccess, aiAccessRequests, users } from '~~/server/db/schema'
import { ensureUserExists } from '~~/server/db/users'
import { getSettings } from '~~/server/utils/serverSettings'

export interface AiAccess {
  hasAccess: boolean
  canSelectModel: boolean
}

// Admin status (see utils/isAdmin.ts) governs the /dashboard + /api/admin/* management surface
// only - it does NOT imply AI feature access. An admin who wants to use the feature themselves
// needs an ai_access row exactly like anyone else (they can grant it to themselves from their own
// dashboard), so the allowlist stays the single source of truth for who can actually generate.
export async function getAiAccess(discordId: string): Promise<AiAccess> {
  if (!getSettings().aiEnabled) return { hasAccess: false, canSelectModel: false }

  const [row] = await getDb().select().from(aiAccess).where(eq(aiAccess.discordId, discordId)).limit(1)

  if (!row) return { hasAccess: false, canSelectModel: false }

  return { hasAccess: true, canSelectModel: row.canSelectModel }
}

export async function grantAccess(
  discordId: string,
  grantedBy: string,
  canSelectModel: boolean,
  note?: string
): Promise<void> {
  // Pre-granting someone who's never connected is allowed (an admin may want to invite someone
  // ahead of time) - ai_access.discord_id has an FK on users.discord_id, so a bare row needs to
  // exist first; upsertUserFromAuth fills in their real name/avatar once they actually connect.
  await ensureUserExists(discordId)

  const now = new Date()
  const record = { canSelectModel, grantedBy, grantedAt: now, note: note ?? null }

  await getDb()
    .insert(aiAccess)
    .values({ discordId, ...record })
    .onConflictDoUpdate({ target: aiAccess.discordId, set: record })
  // Answered.
  await getDb().delete(aiAccessRequests).where(eq(aiAccessRequests.discordId, discordId))
}

export async function revokeAccess(discordId: string): Promise<void> {
  await getDb().delete(aiAccess).where(eq(aiAccess.discordId, discordId))
}

// The account's own daily pool, or the server-wide setting.
export async function dailyCreditsFor(discordId: string): Promise<number> {
  const [row] = await getDb().select({ dailyCredits: aiAccess.dailyCredits }).from(aiAccess).where(eq(aiAccess.discordId, discordId)).limit(1)

  return row?.dailyCredits ?? getSettings().dailyCredits
}

// null: back to the server-wide setting.
export async function setDailyCredits(discordId: string, dailyCredits: number | null): Promise<void> {
  await getDb().update(aiAccess).set({ dailyCredits }).where(eq(aiAccess.discordId, discordId))
}

// After a decline, the same account can ask again a week later.
const DECLINED_WAIT_MS = 7 * 24 * 60 * 60 * 1000

export type AccessRequestStatus = 'none' | 'pending' | 'declined' | 'granted'

export async function accessRequestStatus(discordId: string): Promise<AccessRequestStatus> {
  if ((await getAiAccess(discordId)).hasAccess) return 'granted'

  const [row] = await getDb().select().from(aiAccessRequests).where(eq(aiAccessRequests.discordId, discordId)).limit(1)

  return row?.status ?? 'none'
}

// Asking twice keeps the first request's place in the list. 'too-soon': declined less than a week ago.
export async function requestAccess(discordId: string, message: string | null): Promise<'ok' | 'too-soon'> {
  const db = getDb()
  const [row] = await db.select().from(aiAccessRequests).where(eq(aiAccessRequests.discordId, discordId)).limit(1)

  if (row?.status === 'pending') return 'ok'
  if (row?.status === 'declined' && Date.now() - row.requestedAt.getTime() < DECLINED_WAIT_MS) return 'too-soon'

  const record = { message, status: 'pending' as const, requestedAt: new Date() }

  await db.insert(aiAccessRequests).values({ discordId, ...record }).onConflictDoUpdate({ target: aiAccessRequests.discordId, set: record })

  return 'ok'
}

export async function declineRequest(discordId: string): Promise<void> {
  await getDb().update(aiAccessRequests).set({ status: 'declined', requestedAt: new Date() }).where(eq(aiAccessRequests.discordId, discordId))
}

export async function listPendingRequests() {
  return getDb()
    .select({
      discordId: users.discordId,
      username: users.username,
      displayName: users.displayName,
      avatarUrl: users.avatarUrl,
      message: aiAccessRequests.message,
      requestedAt: aiAccessRequests.requestedAt
    })
    .from(aiAccessRequests)
    .innerJoin(users, eq(users.discordId, aiAccessRequests.discordId))
    .where(eq(aiAccessRequests.status, 'pending'))
    .orderBy(aiAccessRequests.requestedAt)
}

export async function setCanSelectModel(discordId: string, canSelectModel: boolean): Promise<void> {
  await getDb().update(aiAccess).set({ canSelectModel }).where(eq(aiAccess.discordId, discordId))
}

export interface AccessEntry {
  discordId: string
  username: string | null
  displayName: string | null
  avatarUrl: string | null
  lastSeenAt: Date
  hasAccess: boolean
  canSelectModel: boolean
  grantedBy: string | null
  grantedAt: Date | null
  note: string | null
  dailyCredits: number | null
}

// Every known user (users table), left-joined with their ai_access row if any - so the admin
// dashboard can see (and grant access to) someone who's connected but was never invited, not just
// the ones already on the allowlist.
export async function listAccessEntries(): Promise<AccessEntry[]> {
  const rows = await getDb()
    .select({
      discordId: users.discordId,
      username: users.username,
      displayName: users.displayName,
      avatarUrl: users.avatarUrl,
      lastSeenAt: users.lastSeenAt,
      canSelectModel: aiAccess.canSelectModel,
      grantedBy: aiAccess.grantedBy,
      grantedAt: aiAccess.grantedAt,
      note: aiAccess.note,
      dailyCredits: aiAccess.dailyCredits
    })
    .from(users)
    .leftJoin(aiAccess, eq(users.discordId, aiAccess.discordId))
    .orderBy(desc(users.lastSeenAt))

  return rows.map((row) => ({
    ...row,
    hasAccess: row.grantedAt !== null,
    canSelectModel: row.canSelectModel ?? false
  }))
}
