import { and, desc, eq, gte, sql } from 'drizzle-orm'

import { getDb } from '~~/server/db'
import { aiGenerationLog, users } from '~~/server/db/schema'

export type FailureReason = 'access-denied' | 'credit-limit' | 'provider-error'

interface LogAttemptInput {
  discordId: string
  avatarId?: string | null
  avatarName?: string | null
  provider?: string | null
  profile?: string | null
  model?: string | null
  success: boolean
  failureReason?: FailureReason | null
  errorMessage?: string | null
  creditCost?: number | null
  refunded?: boolean
  durationMs?: number | null
}

// One row per generation attempt, success or failure - feeds the admin stats dashboard's
// "worked / why it didn't" reporting (see server/api/admin/stats.get.ts). Full error detail still
// only ever goes to console.error at the call site (see suggest-controls.post.ts); this stores a
// truncated excerpt, since it's meant for at-a-glance display, not full diagnostics.
export async function logGenerationAttempt(input: LogAttemptInput): Promise<void> {
  await getDb()
    .insert(aiGenerationLog)
    .values({
      discordId: input.discordId,
      avatarId: input.avatarId ?? null,
      avatarName: input.avatarName ?? null,
      provider: input.provider ?? null,
      profile: input.profile ?? null,
      model: input.model ?? null,
      success: input.success,
      failureReason: input.failureReason ?? null,
      errorMessage: input.errorMessage ? input.errorMessage.slice(0, 2000) : null,
      creditCost: input.creditCost ?? null,
      refunded: input.refunded ?? false,
      durationMs: input.durationMs ?? null,
      createdAt: new Date()
    })
}

export interface StatsResult {
  daily: { day: string; success: number; failure: number }[]
  byProvider: { provider: string | null; success: number; failure: number }[]
  byProfile: { profile: string | null; count: number }[]
  totalSuccess: number
  totalFailure: number
}

export async function queryStats(days: number): Promise<StatsResult> {
  const db = getDb()
  const since = new Date(Date.now() - days * 24 * 60 * 60 * 1000)
  // createdAt is stored as unix seconds (integer 'timestamp' mode) - date()'s 'unixepoch' modifier
  // matches that directly, no ms conversion needed.
  const dayExpr = sql<string>`date(${aiGenerationLog.createdAt}, 'unixepoch')`
  const successSum = sql<number>`sum(case when ${aiGenerationLog.success} then 1 else 0 end)`
  const failureSum = sql<number>`sum(case when ${aiGenerationLog.success} then 0 else 1 end)`

  const [daily, byProvider, byProfile] = await Promise.all([
    db
      .select({ day: dayExpr, success: successSum, failure: failureSum })
      .from(aiGenerationLog)
      .where(gte(aiGenerationLog.createdAt, since))
      .groupBy(dayExpr)
      .orderBy(dayExpr),
    db
      .select({ provider: aiGenerationLog.provider, success: successSum, failure: failureSum })
      .from(aiGenerationLog)
      .where(gte(aiGenerationLog.createdAt, since))
      .groupBy(aiGenerationLog.provider),
    db
      .select({ profile: aiGenerationLog.profile, count: sql<number>`count(*)` })
      .from(aiGenerationLog)
      .where(gte(aiGenerationLog.createdAt, since))
      .groupBy(aiGenerationLog.profile)
  ])

  const totals = daily.reduce(
    (acc, d) => ({ success: acc.success + Number(d.success), failure: acc.failure + Number(d.failure) }),
    { success: 0, failure: 0 }
  )

  return {
    daily: daily.map((d) => ({ day: d.day, success: Number(d.success), failure: Number(d.failure) })),
    byProvider: byProvider.map((p) => ({ provider: p.provider, success: Number(p.success), failure: Number(p.failure) })),
    byProfile: byProfile.map((p) => ({ profile: p.profile, count: Number(p.count) })),
    totalSuccess: totals.success,
    totalFailure: totals.failure
  }
}

export interface RecentAttemptFilters {
  discordId?: string
  provider?: string
  success?: boolean
}

export async function listRecentAttempts(filters: RecentAttemptFilters, page = 1, pageSize = 25) {
  const conditions = []

  if (filters.discordId) conditions.push(eq(aiGenerationLog.discordId, filters.discordId))
  if (filters.provider) conditions.push(eq(aiGenerationLog.provider, filters.provider))
  if (filters.success !== undefined) conditions.push(eq(aiGenerationLog.success, filters.success))

  return getDb()
    .select({
      id: aiGenerationLog.id,
      discordId: aiGenerationLog.discordId,
      displayName: users.displayName,
      avatarName: aiGenerationLog.avatarName,
      provider: aiGenerationLog.provider,
      profile: aiGenerationLog.profile,
      model: aiGenerationLog.model,
      success: aiGenerationLog.success,
      failureReason: aiGenerationLog.failureReason,
      errorMessage: aiGenerationLog.errorMessage,
      refunded: aiGenerationLog.refunded,
      createdAt: aiGenerationLog.createdAt
    })
    .from(aiGenerationLog)
    .leftJoin(users, eq(users.discordId, aiGenerationLog.discordId))
    .where(conditions.length ? and(...conditions) : undefined)
    .orderBy(desc(aiGenerationLog.createdAt))
    .limit(pageSize)
    .offset((page - 1) * pageSize)
}
