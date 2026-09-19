import { desc, eq } from 'drizzle-orm'

import { getDb } from '~~/server/db'
import { aiAccess, users } from '~~/server/db/schema'
import { ensureUserExists } from '~~/server/db/users'

export interface AiAccess {
  hasAccess: boolean
  canSelectModel: boolean
}

// Admin status (see utils/isAdmin.ts) governs the /dashboard + /api/admin/* management surface
// only - it does NOT imply AI feature access. An admin who wants to use the feature themselves
// needs an ai_access row exactly like anyone else (they can grant it to themselves from their own
// dashboard), so the allowlist stays the single source of truth for who can actually generate.
export async function getAiAccess(discordId: string): Promise<AiAccess> {
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
}

export async function revokeAccess(discordId: string): Promise<void> {
  await getDb().delete(aiAccess).where(eq(aiAccess.discordId, discordId))
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
      note: aiAccess.note
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
