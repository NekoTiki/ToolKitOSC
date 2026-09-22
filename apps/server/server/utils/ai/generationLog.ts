import { and, desc, eq, gte, sql } from 'drizzle-orm'

import { getDb } from '~~/server/db'
import { aiGenerationLog, users } from '~~/server/db/schema'
import { notifyAdmins } from '~~/server/routes/admin/ws'
import { getAiProvider } from '~~/server/utils/ai'
import { getPromptProfile } from '~~/server/utils/ai/profiles'

export type FailureReason = 'access-denied' | 'credit-limit' | 'provider-error'

// Single source of truth for how a failure reason reads on the admin dashboard (see
// GenerationLogEntry.failureReasonLabel below) - a plain enum value like 'credit-limit' is fine as
// a stable, filterable category (see RecentAttemptFilters), but isn't what should be shown to a
// human reading the "recent generations" log.
const FAILURE_REASON_LABELS: Record<FailureReason, string> = {
  'access-denied': 'Access denied',
  'credit-limit': 'Credit limit reached',
  'provider-error': 'Provider error'
}

export function failureReasonLabel(reason: string | null): string | null {
  return reason && reason in FAILURE_REASON_LABELS ? FAILURE_REASON_LABELS[reason as FailureReason] : reason
}

// getAiProvider()/getPromptProfile() always resolve a label regardless of whether that provider is
// currently configured/that profile still exists (see index.ts/profiles.ts) - a historical row for
// a provider that's since been removed from the server's env just falls back to its raw id, rather
// than the whole row failing to resolve. Exported: also used by liveGenerations.ts, for the exact
// same labels on an in-progress row as a finished one gets.
export function providerLabel(provider: string | null): string | null {
  return provider ? (getAiProvider(provider)?.label ?? provider) : provider
}

export function profileLabel(profile: string | null): string | null {
  return profile ? (getPromptProfile(profile)?.label ?? profile) : profile
}

interface LogAttemptInput {
  // Only set for an attempt that went through the async runGeneration path (see
  // suggest-controls.post.ts) - an early synchronous failure (access-denied, credit-limit) never
  // had a live placeholder to begin with (see liveGenerations.ts), so there's nothing to
  // correlate for those and this is left undefined.
  requestId?: string
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
  // Only ever set on the live 'generation-logged' WS push, never on a plain REST-listed row (see
  // requestId's own comment above - it isn't a column in ai_generation_log, so a page fetched via
  // GET /api/admin/attempts never has one). The dashboard uses it to drop this attempt's live
  // placeholder (see liveGenerations.ts) right as the real row replaces it.
  requestId?: string
  discordId: string
  displayName: string | null
  avatarName: string | null
  provider: string | null
  // Human-readable form of `provider`/`profile`/`failureReason` (see providerLabel()/
  // profileLabel()/failureReasonLabel() above) - the raw values are kept alongside them since
  // they're still what RecentAttemptFilters/byProvider/byProfile filter and group by.
  providerLabel: string | null
  profile: string | null
  profileLabel: string | null
  model: string | null
  success: boolean
  failureReason: string | null
  failureReasonLabel: string | null
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
      requestId: input.requestId,
      discordId: input.discordId,
      displayName: user?.displayName ?? null,
      avatarName: input.avatarName ?? null,
      provider: input.provider ?? null,
      providerLabel: providerLabel(input.provider ?? null),
      profile: input.profile ?? null,
      profileLabel: profileLabel(input.profile ?? null),
      model: input.model ?? null,
      success: input.success,
      failureReason: input.failureReason ?? null,
      failureReasonLabel: failureReasonLabel(input.failureReason ?? null),
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

// `discordId` narrows every query to one account's own attempts - used as-is (no filter) for the
// global /dashboard/stats charts, and passed by the per-user drill-down page (see
// api/admin/users/[discordId]/stats.get.ts) to get the exact same shape scoped to just them.
export async function queryStats(days: number, discordId?: string): Promise<StatsResult> {
  const db = getDb()
  const since = new Date(Date.now() - days * 24 * 60 * 60 * 1000)
  const sinceCondition = discordId
    ? and(gte(aiGenerationLog.createdAt, since), eq(aiGenerationLog.discordId, discordId))
    : gte(aiGenerationLog.createdAt, since)
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
      .where(sinceCondition)
      .groupBy(dayExpr)
      .orderBy(dayExpr),
    db
      .select({ provider: aiGenerationLog.provider, success: successSum, failure: failureSum, avgDurationMs: avgDurationExpr })
      .from(aiGenerationLog)
      .where(sinceCondition)
      .groupBy(aiGenerationLog.provider),
    db
      .select({ profile: aiGenerationLog.profile, count: sql<number>`count(*)` })
      .from(aiGenerationLog)
      .where(sinceCondition)
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

export async function listRecentAttempts(filters: RecentAttemptFilters, page = 1, pageSize = 25): Promise<GenerationLogEntry[]> {
  const conditions = []

  if (filters.discordId) conditions.push(eq(aiGenerationLog.discordId, filters.discordId))
  if (filters.provider) conditions.push(eq(aiGenerationLog.provider, filters.provider))
  if (filters.success !== undefined) conditions.push(eq(aiGenerationLog.success, filters.success))

  const rows = await getDb()
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

  return rows.map((row) => ({
    ...row,
    providerLabel: providerLabel(row.provider),
    profileLabel: profileLabel(row.profile),
    failureReasonLabel: failureReasonLabel(row.failureReason),
    createdAt: row.createdAt.toISOString()
  }))
}
