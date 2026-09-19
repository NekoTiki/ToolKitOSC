import { index, integer, primaryKey, sqliteTable, text } from 'drizzle-orm/sqlite-core'

// General-purpose Discord identity cache, not AI-specific - upserted on every successful host WS
// auth (see routes/host.ts) and on every authenticated AI REST call (see api/ai/suggest-controls),
// the latter so ai_access/ai_credit_*/ai_generation_log rows always have a valid FK target even
// for a desktop client that calls the REST API without ever opening a host WS connection.
export const users = sqliteTable('users', {
  discordId: text('discord_id').primaryKey(),
  username: text('username'),
  displayName: text('display_name'),
  avatarUrl: text('avatar_url'),
  lastSeenAt: integer('last_seen_at', { mode: 'timestamp' }).notNull()
})

// Row presence = has AI access - the allowlist for every account, admins included (see
// utils/isAdmin.ts's separate, unrelated dashboard-management gate).
export const aiAccess = sqliteTable('ai_access', {
  discordId: text('discord_id')
    .primaryKey()
    .references(() => users.discordId),
  canSelectModel: integer('can_select_model', { mode: 'boolean' }).notNull().default(false),
  grantedBy: text('granted_by').notNull(),
  grantedAt: integer('granted_at', { mode: 'timestamp' }).notNull(),
  note: text('note')
})

// Day-partitioned (UTC calendar day, 'YYYY-MM-DD') rather than a rolling 24h window - a new day
// starts every counter at zero with no explicit reset write needed, and survives server restarts
// exactly instead of drifting off whatever moment the window happened to start. One pool per
// account per day, shared across every provider (not per-provider) - a single overall daily budget.
export const aiCreditUsage = sqliteTable(
  'ai_credit_usage',
  {
    discordId: text('discord_id').notNull(),
    day: text('day').notNull(),
    spent: integer('spent').notNull().default(0)
  },
  (t) => [primaryKey({ columns: [t.discordId, t.day] })]
)

// Admin "increase credits for today" top-ups (see api/admin/users/[discordId]/credits.post.ts) -
// additive on top of ACCOUNT_DAILY_CREDITS for that (discordId, day), not a replacement.
export const aiCreditBonus = sqliteTable(
  'ai_credit_bonus',
  {
    discordId: text('discord_id').notNull(),
    day: text('day').notNull(),
    bonus: integer('bonus').notNull().default(0)
  },
  (t) => [primaryKey({ columns: [t.discordId, t.day] })]
)

// One row per generation attempt, success or failure - feeds the admin stats dashboard's "worked /
// why it didn't" reporting. `failureReason` is a stable, filterable category; `errorMessage` is a
// truncated detail string for display (full detail still only ever goes to console.error server-side).
export const aiGenerationLog = sqliteTable(
  'ai_generation_log',
  {
    id: integer('id').primaryKey({ autoIncrement: true }),
    discordId: text('discord_id').notNull(),
    avatarId: text('avatar_id'),
    avatarName: text('avatar_name'),
    provider: text('provider'),
    profile: text('profile'),
    model: text('model'),
    success: integer('success', { mode: 'boolean' }).notNull(),
    failureReason: text('failure_reason'),
    errorMessage: text('error_message'),
    creditCost: integer('credit_cost'),
    // True when creditCost was actually spent and then given back (see rateLimit.ts's
    // refundAccountCredits) because the provider rejected the request before any generation ran -
    // kept as an explicit flag rather than zeroing creditCost, so the log still shows what a
    // generation at this profile would normally have cost.
    refunded: integer('refunded', { mode: 'boolean' }).notNull().default(false),
    durationMs: integer('duration_ms'),
    createdAt: integer('created_at', { mode: 'timestamp' }).notNull()
  },
  (t) => [index('ai_generation_log_created_at_idx').on(t.createdAt), index('ai_generation_log_discord_id_idx').on(t.discordId)]
)
