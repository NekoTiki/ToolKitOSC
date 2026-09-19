import { and, eq } from 'drizzle-orm'

import { getDb } from '~~/server/db'
import { aiCreditBonus, aiCreditUsage } from '~~/server/db/schema'

// UTC calendar day, not a rolling 24h window - every counter below is keyed by this, so a new day
// starts everything at zero with no explicit reset write needed (see server/plugins/dailyMaintenance.ts
// for the actual daily job, which only prunes old rows).
function today(): string {
  return new Date().toISOString().slice(0, 10)
}

async function creditBonus(discordId: string, day: string): Promise<number> {
  const [row] = await getDb()
    .select({ bonus: aiCreditBonus.bonus })
    .from(aiCreditBonus)
    .where(and(eq(aiCreditBonus.discordId, discordId), eq(aiCreditBonus.day, day)))
    .limit(1)

  return row?.bonus ?? 0
}

async function creditSpent(discordId: string, day: string): Promise<number> {
  const [row] = await getDb()
    .select({ spent: aiCreditUsage.spent })
    .from(aiCreditUsage)
    .where(and(eq(aiCreditUsage.discordId, discordId), eq(aiCreditUsage.day, day)))
    .limit(1)

  return row?.spent ?? 0
}

// Read-only - for reporting remaining quota (GET /api/ai/providers) without spending anything.
export async function remainingAccountCredits(discordId: string, basePoolSize: number): Promise<number> {
  const day = today()
  const [spent, bonus] = await Promise.all([creditSpent(discordId, day), creditBonus(discordId, day)])

  return Math.max(0, basePoolSize + bonus - spent)
}

// One overall daily credit pool per account, shared across every provider - not per-(account,
// provider) - spent at whatever rate the calling profile costs (see profiles.ts's `creditCost`),
// fungible across profiles AND providers. `basePoolSize` is ACCOUNT_DAILY_CREDITS; an admin's
// addCreditBonus top-up for today is added on top of it here. Returns the fresh remaining count on
// success, or null if `cost` doesn't fit what's left today - not just increment-and-report, so a
// caller can't race past the pool with repeat calls.
export async function consumeAccountCredits(discordId: string, cost: number, basePoolSize: number): Promise<number | null> {
  const day = today()
  const [spent, bonus] = await Promise.all([creditSpent(discordId, day), creditBonus(discordId, day)])
  const poolSize = basePoolSize + bonus

  if (spent + cost > poolSize) return null

  const newSpent = spent + cost

  await getDb()
    .insert(aiCreditUsage)
    .values({ discordId, day, spent: newSpent })
    .onConflictDoUpdate({
      target: [aiCreditUsage.discordId, aiCreditUsage.day],
      set: { spent: newSpent }
    })

  return poolSize - newSpent
}

// Un-spends credits after a request that got charged but turned out to never actually run a
// generation (see providerError.ts's AiRefundableError) - reduces today's `spent`, not a bonus
// top-up, so it stays out of the admin-granted-credits audit trail (addCreditBonus below) and
// tomorrow's fresh pool isn't affected either way. Floors at 0 instead of going negative if this
// is ever called for more than what's actually recorded as spent today. Returns the fresh
// remaining count, same as consumeAccountCredits, so the caller can push a live WS update with it.
export async function refundAccountCredits(discordId: string, amount: number, basePoolSize: number): Promise<number> {
  const day = today()
  const [spent, bonus] = await Promise.all([creditSpent(discordId, day), creditBonus(discordId, day)])
  const newSpent = Math.max(0, spent - amount)

  await getDb()
    .insert(aiCreditUsage)
    .values({ discordId, day, spent: newSpent })
    .onConflictDoUpdate({
      target: [aiCreditUsage.discordId, aiCreditUsage.day],
      set: { spent: newSpent }
    })

  return basePoolSize + bonus - newSpent
}

// Admin "increase credits for today" (see api/admin/users/[discordId]/credits.post.ts) - additive,
// not a replacement, and only ever applies to today's UTC day (tomorrow starts fresh again).
export async function addCreditBonus(discordId: string, amount: number): Promise<void> {
  const day = today()
  const current = await creditBonus(discordId, day)
  const newBonus = current + amount

  await getDb()
    .insert(aiCreditBonus)
    .values({ discordId, day, bonus: newBonus })
    .onConflictDoUpdate({
      target: [aiCreditBonus.discordId, aiCreditBonus.day],
      set: { bonus: newBonus }
    })
}
