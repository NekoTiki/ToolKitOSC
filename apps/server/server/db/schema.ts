import { sql } from 'drizzle-orm'
import { index, integer, primaryKey, sqliteTable, text, uniqueIndex } from 'drizzle-orm/sqlite-core'

// General-purpose Discord identity cache, not AI-specific - upserted on every successful host WS
// auth (see routes/host.ts) and on every authenticated AI REST call (see api/ai/suggest-controls),
// the latter so ai_access/ai_credit_*/ai_generation_log rows always have a valid FK target even
// for a desktop client that calls the REST API without ever opening a host WS connection.
export const users = sqliteTable('users', {
  discordId: text('discord_id').primaryKey(),
  username: text('username'),
  displayName: text('display_name'),
  avatarUrl: text('avatar_url'),
  lastSeenAt: integer('last_seen_at', { mode: 'timestamp' }).notNull(),
  // Whether the old share links built from the Discord id itself (/share/<id>, /s/<base62 of id>)
  // still open the page. Turned off for good when the host resets their link.
  legacyShareLink: integer('legacy_share_link', { mode: 'boolean' }).notNull().default(true),
  // Set by an admin (see db/accounts.ts): until then the app can't sign in and share pages treat
  // them as a guest. Null when not suspended.
  suspendedUntil: integer('suspended_until', { mode: 'timestamp' }),
  suspendReason: text('suspend_reason'),
  // Desktop tokens carry the version they were made with (`tv`); bumping it signs out every app.
  tokenVersion: integer('token_version').notNull().default(0),
  // What the desktop app last signed in with, for the admin dashboard. Null for older apps.
  appVersion: text('app_version'),
  os: text('os'),
  // Shown on the share page while the host is offline ("Back Friday 9 pm"), set in Settings > Sharing.
  offlineMessage: text('offline_message'),
  // When the host's app last disconnected, for "last live 2 h ago".
  lastOnlineAt: integer('last_online_at', { mode: 'timestamp' }),
  // The host's "Only people you let in", kept so it still holds after a server restart until
  // their app is back and sends who they let in.
  shareApproval: integer('share_approval', { mode: 'boolean' }).notNull().default(false)
})

// Settings admins change from the dashboard (see utils/serverSettings.ts), one row per key, the
// value as JSON. A key with no row uses its default.
export const serverSettings = sqliteTable('server_settings', {
  key: text('key').primaryKey(),
  value: text('value', { mode: 'json' }).notNull()
})

// How busy the relay was each UTC day (see utils/relayStats.ts). Pruned with the other stats.
export const relayDaily = sqliteTable('relay_daily', {
  day: text('day').primaryKey(),
  peakHosts: integer('peak_hosts').notNull().default(0),
  peakViewers: integer('peak_viewers').notNull().default(0),
  joins: integer('joins').notNull().default(0),
  rateLimited: integer('rate_limited').notNull().default(0)
})

// What admins did from the dashboard, for the user page and the Log page. Never pruned: it's the
// record of who changed what, and stays small. `detail` is JSON (the reason, amount, text...).
export const adminLog = sqliteTable(
  'admin_log',
  {
    id: integer('id').primaryKey({ autoIncrement: true }),
    adminId: text('admin_id').notNull(),
    action: text('action').notNull(),
    targetId: text('target_id'),
    detail: text('detail', { mode: 'json' }),
    createdAt: integer('created_at', { mode: 'timestamp' }).notNull()
  },
  (t) => [index('admin_log_target_id_idx').on(t.targetId), index('admin_log_created_at_idx').on(t.createdAt)]
)

// Codes that open a host's share page (see db/shareLinks.ts). `main` is the random link everyone
// gets, `custom` the name the host picked (at most one each), `invite` a link that can expire.
export const shareLinks = sqliteTable(
  'share_links',
  {
    code: text('code').primaryKey(),
    discordId: text('discord_id')
      .notNull()
      .references(() => users.discordId),
    kind: text('kind', { enum: ['main', 'custom', 'invite'] }).notNull(),
    expiresAt: integer('expires_at', { mode: 'timestamp' }),
    createdAt: integer('created_at', { mode: 'timestamp' }).notNull()
  },
  (table) => [
    index('share_links_discord_id_idx').on(table.discordId),
    uniqueIndex('share_links_one_main_and_custom').on(table.discordId, table.kind).where(sql`${table.kind} != 'invite'`)
  ]
)

// Row presence = has AI access - the allowlist for every account, admins included (see
// utils/isAdmin.ts's separate, unrelated dashboard-management gate).
export const aiAccess = sqliteTable('ai_access', {
  discordId: text('discord_id')
    .primaryKey()
    .references(() => users.discordId),
  canSelectModel: integer('can_select_model', { mode: 'boolean' }).notNull().default(false),
  grantedBy: text('granted_by').notNull(),
  grantedAt: integer('granted_at', { mode: 'timestamp' }).notNull(),
  note: text('note'),
  // This account's daily credit pool instead of the server-wide setting. Null: the setting.
  dailyCredits: integer('daily_credits')
})

// "Request access" from the app's AI page, listed for admins on the Users page. Granting access
// removes the row; declining keeps it, so the same account can't ask again right away.
export const aiAccessRequests = sqliteTable('ai_access_requests', {
  discordId: text('discord_id')
    .primaryKey()
    .references(() => users.discordId),
  message: text('message'),
  status: text('status', { enum: ['pending', 'declined'] }).notNull(),
  requestedAt: integer('requested_at', { mode: 'timestamp' }).notNull()
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
// additive on top of the dailyCredits setting for that (discordId, day), not a replacement.
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

// Snapshot of how many controls a host currently has configured for one avatar, by type - replaced
// wholesale for that (discordId, avatarId) pair every time the host sends a `controls-update` (see
// routes/host.ts and utils/controlStats.ts), mirroring how the in-memory `hostControls` map itself
// is replaced wholesale on that same event. Keyed by avatarId (not just discordId) because controls
// are authored per-avatar client-side (see apps/client's useControls.ts) - a `controls-update` only
// ever reports whichever avatar is currently loaded, so without this an account's inventory would
// only ever reflect one avatar at a time instead of every avatar they've configured controls for.
// A type with zero controls simply has no row here, rather than a row with count 0, so a total is a
// plain sum with nothing to filter out.
export const controlInventory = sqliteTable(
  'control_inventory',
  {
    discordId: text('discord_id').notNull(),
    avatarId: text('avatar_id').notNull(),
    type: text('type').notNull(),
    count: integer('count').notNull().default(0),
    updatedAt: integer('updated_at', { mode: 'timestamp' }).notNull()
  },
  (t) => [primaryKey({ columns: [t.discordId, t.avatarId, t.type] })]
)

// Day-partitioned (UTC calendar day) activation counters, one row per (discordId, type, day) -
// same reasoning as ai_credit_usage above. `discordId` here is the *host's* room id (whoever owns
// the control panel a command executed on), not the viewer who sent it - see routes/ws/[ws].ts,
// where a command is counted against the room it was relayed into.
export const controlActivationDaily = sqliteTable(
  'control_activation_daily',
  {
    discordId: text('discord_id').notNull(),
    type: text('type').notNull(),
    day: text('day').notNull(),
    count: integer('count').notNull().default(0)
  },
  (t) => [
    primaryKey({ columns: [t.discordId, t.type, t.day] }),
    index('control_activation_daily_discord_id_idx').on(t.discordId)
  ]
)
