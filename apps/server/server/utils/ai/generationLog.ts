import { and, desc, eq, gte, sql } from 'drizzle-orm'

import { getDb } from '~~/server/db'
import { aiGenerationLog, users } from '~~/server/db/schema'
import { notifyAdmins } from '~~/server/routes/admin/ws'

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

// Shape of a single row as the admin stats dashboard displays it - shared between
// listRecentAttempts' select below and the live WS push, so the one that arrives over
// 'generation-logged' can be spliced straight into the same list a REST fetch would return.
export interface GenerationLogEntry {
  id: number
  discordId: string
  displayName: string | null
  avatarName: string | null
  provider: string | null
  profile: string | null
  model: string | null
  success: boolean
  failureReason: string | null
  errorMessage: string | null
  refunded: boolean
  durationMs: number | null
  createdAt: string
}

// One row per generation attempt, success or failure - feeds the admin stats dashboard's
// "worked / why it didn't" reporting (see server/api/admin/stats.get.ts). Full error detail still
// only ever goes to console.error at the call site (see suggest-controls.post.ts); this stores a
// truncated excerpt, since it's meant for at-a-glance display, not full diagnostics.
export async function logGenerationAttempt(input: LogAttemptInput): Promise<void> {
  const db = getDb()
  const createdAt = new Date()

  const [row] = await db
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
      createdAt
    })
    .returning({ id: aiGenerationLog.id })

  // insert().returning() always yields exactly one row for a single-object .values() call - drizzle
  // just types it as an array since some dialects/call shapes can return more than one.
  if (!row) throw new Error('Insert into ai_generation_log returned no row')

  // Pushed as the actual row, not just a "something changed" signal - lets the dashboard splice it
  // straight into its existing state instead of re-running the aggregate/list queries on every
  // single attempt (see stats.vue's applyIncrementalUpdate). displayName needs its own lookup since
  // it isn't part of this table - a single indexed row read, cheap next to everything else here.
  const [user] = await db.select({ displayName: users.displayName }).from(users).where(eq(users.discordId, input.discordId)).limit(1)

  notifyAdmins({
    type: 'generation-logged',
    attempt: {
      id: row.id,
      discordId: input.discordId,
      displayName: user?.displayName ?? null,
      avatarName: input.avatarName ?? null,
      provider: input.provider ?? null,
      profile: input.profile ?? null,
      model: input.model ?? null,
      success: input.success,
      failureReason: input.failureReason ?? null,
      errorMessage: input.errorMessage ? input.errorMessage.slice(0, 2000) : null,
      refunded: input.refunded ?? false,
      durationMs: input.durationMs ?? null,
      createdAt: createdAt.toISOString()
    } satisfies GenerationLogEntry
  })
}

export interface StatsResult {
  daily: { day: string; success: number; failure: number }[]
  byProvider: { provider: string | null; success: number; failure: number; avgDurationMs: number | null }[]
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
  // Successful attempts only - a failed one's duration is "how long until it errored out", not a
  // real generation time, and would only drag the average down. Can't just add a WHERE clause for
  // this (that would also drop failed rows from the success/failure counts below), so the case
  // expression nulls out everything else instead - avg() already ignores NULL on its own, same as
  // it does for the access-denied/credit-limit failures that never got as far as starting a timer
  // (see suggest-controls.post.ts).
  const avgDurationExpr = sql<number | null>`avg(case when ${aiGenerationLog.success} then ${aiGenerationLog.durationMs} else null end)`

  const [daily, byProvider, byProfile] = await Promise.all([
    db
      .select({ day: dayExpr, success: successSum, failure: failureSum })
      .from(aiGenerationLog)
      .where(gte(aiGenerationLog.createdAt, since))
      .groupBy(dayExpr)
      .orderBy(dayExpr),
    db
      .select({ provider: aiGenerationLog.provider, success: successSum, failure: failureSum, avgDurationMs: avgDurationExpr })
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
    byProvider: byProvider.map((p) => ({
      provider: p.provider,
      success: Number(p.success),
      failure: Number(p.failure),
      avgDurationMs: p.avgDurationMs === null ? null : Number(p.avgDurationMs)
    })),
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
      durationMs: aiGenerationLog.durationMs,
      createdAt: aiGenerationLog.createdAt
    })
    .from(aiGenerationLog)
    .leftJoin(users, eq(users.discordId, aiGenerationLog.discordId))
    .where(conditions.length ? and(...conditions) : undefined)
    .orderBy(desc(aiGenerationLog.createdAt))
    .limit(pageSize)
    .offset((page - 1) * pageSize)
}
