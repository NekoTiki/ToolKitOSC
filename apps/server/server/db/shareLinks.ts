import { randomBytes } from 'node:crypto'

import { and, eq } from 'drizzle-orm'

import { decodeShareCode } from '#shared/utils/shortLink'

import { getDb } from './index'
import { shareLinks, users } from './schema'

// A share page's room is still keyed by the host's Discord id inside the server (see
// routes/host.ts). The code in the link is only translated to it here, where a link comes in.
export async function resolveShareCode(code: string): Promise<string | null> {
  const db = getDb()
  const [link] = await db.select().from(shareLinks).where(eq(shareLinks.code, code)).limit(1)

  if (link) return !link.expiresAt || link.expiresAt > new Date() ? link.discordId : null

  // Links from before share codes: the raw Discord id, or its base62 short form. They work until
  // the host resets their link.
  const discordId = /^\d{17,20}$/.test(code) ? code : decodeShareCode(code)

  if (!discordId) return null

  const [user] = await db.select({ legacy: users.legacyShareLink }).from(users).where(eq(users.discordId, discordId)).limit(1)

  return user?.legacy ? discordId : null
}

const newCode = (): string => randomBytes(8).toString('base64url')

// The host's link, made the first time they ask for it. Their user row exists by then: it's
// upserted on every sign-in (see db/users.ts).
export async function mainShareCode(discordId: string): Promise<string> {
  const db = getDb()
  const current = () =>
    db
      .select({ code: shareLinks.code })
      .from(shareLinks)
      .where(and(eq(shareLinks.discordId, discordId), eq(shareLinks.kind, 'main')))
      .limit(1)

  const [link] = await current()

  if (link) return link.code

  // Two requests at once both land here; the unique index keeps one and the other reads it back.
  await db.insert(shareLinks).values({ code: newCode(), discordId, kind: 'main', createdAt: new Date() }).onConflictDoNothing()

  const [created] = await current()

  return created!.code
}

// Kills the current link and the old Discord-id ones; they 404 from now on.
export async function resetMainShareCode(discordId: string): Promise<string> {
  const db = getDb()

  await db.transaction(async (tx) => {
    await tx.delete(shareLinks).where(and(eq(shareLinks.discordId, discordId), eq(shareLinks.kind, 'main')))
    await tx.update(users).set({ legacyShareLink: false }).where(eq(users.discordId, discordId))
  })

  return mainShareCode(discordId)
}
