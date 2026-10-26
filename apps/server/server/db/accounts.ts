import { eq, sql } from 'drizzle-orm'

import { getDb } from './index'
import {
  aiAccess,
  aiAccessRequests,
  aiCreditBonus,
  aiCreditUsage,
  aiGenerationLog,
  controlActivationDaily,
  controlInventory,
  shareLinks,
  users
} from './schema'

// Suspensions and "sign out everywhere", checked wherever a Discord account signs in: the desktop
// app's /host socket and REST calls (with the token's `tv`), share-link routes and share pages.

const SUSPENDED_FOREVER = new Date('9999-12-31T00:00:00Z')

const accountOf = async (discordId: string) => {
  const [user] = await getDb()
    .select({ suspendedUntil: users.suspendedUntil, suspendReason: users.suspendReason, tokenVersion: users.tokenVersion })
    .from(users)
    .where(eq(users.discordId, discordId))
    .limit(1)

  return user
}

export async function tokenVersionOf(discordId: string): Promise<number> {
  return (await accountOf(discordId))?.tokenVersion ?? 0
}

export const suspensionText = (until: Date, reason: string | null): string =>
  `Your account is suspended ${until >= SUSPENDED_FOREVER ? 'until further notice' : `until ${until.toISOString().slice(0, 16).replace('T', ' ')} UTC`}${reason ? `: ${reason}` : '.'}`

// Why this account can't sign in right now, or null. `tokenVersion` is the desktop token's `tv`
// (missing on tokens made before it existed: 0); leave it out for a browser session.
export async function accountProblem(discordId: string, tokenVersion?: number): Promise<string | null> {
  const account = await accountOf(discordId)

  if (!account) return null

  if (account.suspendedUntil && account.suspendedUntil > new Date()) return suspensionText(account.suspendedUntil, account.suspendReason)

  if (tokenVersion !== undefined && tokenVersion < account.tokenVersion) return 'You were signed out. Sign in again.'

  return null
}

export async function isSuspended(discordId: string): Promise<boolean> {
  const until = (await accountOf(discordId))?.suspendedUntil

  return !!until && until > new Date()
}

// `days` null: until an admin lifts it.
export async function suspend(discordId: string, days: number | null, reason: string | null): Promise<string> {
  const until = days ? new Date(Date.now() + days * 86_400_000) : SUSPENDED_FOREVER

  await getDb().update(users).set({ suspendedUntil: until, suspendReason: reason }).where(eq(users.discordId, discordId))

  return suspensionText(until, reason)
}

export async function unsuspend(discordId: string): Promise<void> {
  await getDb().update(users).set({ suspendedUntil: null, suspendReason: null }).where(eq(users.discordId, discordId))
}

// Everything the server keeps about this account ("Delete my data"). The admin log stays: it's
// the admins' record of what they did. Their share links stop working.
export async function deleteAccountData(discordId: string): Promise<void> {
  const db = getDb()

  await db.batch([
    db.delete(shareLinks).where(eq(shareLinks.discordId, discordId)),
    db.delete(aiAccess).where(eq(aiAccess.discordId, discordId)),
    db.delete(aiAccessRequests).where(eq(aiAccessRequests.discordId, discordId)),
    db.delete(aiCreditUsage).where(eq(aiCreditUsage.discordId, discordId)),
    db.delete(aiCreditBonus).where(eq(aiCreditBonus.discordId, discordId)),
    db.delete(aiGenerationLog).where(eq(aiGenerationLog.discordId, discordId)),
    db.delete(controlInventory).where(eq(controlInventory.discordId, discordId)),
    db.delete(controlActivationDaily).where(eq(controlActivationDaily.discordId, discordId)),
    // Last: the rows above point at it.
    db.delete(users).where(eq(users.discordId, discordId))
  ])
}

export async function signOutEverywhere(discordId: string): Promise<void> {
  await getDb().update(users).set({ tokenVersion: sql`${users.tokenVersion} + 1` }).where(eq(users.discordId, discordId))
}
