import { like, or } from 'drizzle-orm'

import { getDb } from './index'
import { users } from './schema'

interface UpsertUserInput {
  discordId: string
  username?: string | null
  displayName?: string | null
  avatarUrl?: string | null
}

async function upsertUser(input: UpsertUserInput): Promise<void> {
  const db = getDb()
  const now = new Date()
  const record = {
    username: input.username ?? null,
    displayName: input.displayName ?? null,
    avatarUrl: input.avatarUrl ?? null,
    lastSeenAt: now
  }

  await db
    .insert(users)
    .values({ discordId: input.discordId, ...record })
    .onConflictDoUpdate({ target: users.discordId, set: record })
}

// Adapts the two shapes the JWT's `user` field actually takes across call sites (routes/host.ts's
// `User` from #auth-utils, verifyDesktopToken's `DesktopUser`) - both carry the same
// `discord: {id, avatar, name}` + top-level `username` fields, just as differently-named types.
export async function upsertUserFromAuth(user: {
  discord?: { id: string; avatar: string; name: string }
  username?: string
}): Promise<void> {
  if (!user.discord?.id) return

  await upsertUser({
    discordId: user.discord.id,
    username: user.username,
    displayName: user.discord.name,
    avatarUrl: user.discord.avatar
  })
}

// Used when granting AI access to a Discord id that hasn't connected yet (pre-granting) - inserts
// a bare row with no name/avatar so ai_access's FK has something to point at; upsertUserFromAuth
// fills the real name/avatar in once that user actually connects.
export async function ensureUserExists(discordId: string): Promise<void> {
  await getDb().insert(users).values({ discordId, lastSeenAt: new Date() }).onConflictDoNothing()
}

export async function searchUsers(query: string, limit = 20) {
  const db = getDb()
  const like_ = `%${query}%`

  return db
    .select()
    .from(users)
    .where(or(like(users.discordId, like_), like(users.username, like_), like(users.displayName, like_)))
    .limit(limit)
}
