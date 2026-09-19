// Two independent limits, not one - a user could otherwise burn their whole daily budget
// re-rolling the same avatar over and over, or spread it thin across many. In-memory, day-window
// counters (mirrors server/utils/rateLimiter.ts's fixed-window approach, just a day instead of a
// second) - resets on server restart, which is an accepted tradeoff here the same way it already
// is for that module, not something this adds new.
const DAY_MS = 24 * 60 * 60 * 1000

// A user can generate for the same avatar at most this many times a day, regardless of which
// provider/profile they switch between - independent of the account credit pool below, and not
// itself profile-costed (see the project's conceptual writeup: this is meant as exactly two
// knobs, not a cross product with every profile) - a flat anti-spam count, not a resource cost.
export const AVATAR_DAILY_LIMIT = 3

interface Bucket {
  spent: number
  windowStart: number
}

const buckets = new Map<string, Bucket>()

// Returns the remaining pool size if `cost` fits within what's left today (and spends it), or
// null if it doesn't - checked, not just incremented-and-reported, so a caller can't accidentally
// race past the pool by calling this more than once for what should be a single consumption.
function consume(key: string, poolSize: number, cost: number): number | null {
  const now = Date.now()
  const bucket = buckets.get(key)

  if (!bucket || now - bucket.windowStart >= DAY_MS) {
    if (cost > poolSize) return null

    buckets.set(key, { spent: cost, windowStart: now })

    return poolSize - cost
  }

  if (bucket.spent + cost > poolSize) return null

  bucket.spent += cost

  return poolSize - bucket.spent
}

// Read-only - for reporting remaining quota (see GET /api/ai/options) without spending anything.
function remaining(key: string, poolSize: number): number {
  const bucket = buckets.get(key)

  if (!bucket || Date.now() - bucket.windowStart >= DAY_MS) return poolSize

  return Math.max(0, poolSize - bucket.spent)
}

// Shared per-(account, provider) credit pool, spent at whatever rate the calling profile costs
// (see profiles.ts's `creditCost`) - fungible across profiles, unlike the old per-profile caps.
export function consumeAccountCredits(
  discordUserId: string,
  provider: string,
  cost: number,
  poolSize: number
): number | null {
  return consume(`account:${discordUserId}:${provider}`, poolSize, cost)
}

export function remainingAccountCredits(discordUserId: string, provider: string, poolSize: number): number {
  return remaining(`account:${discordUserId}:${provider}`, poolSize)
}

export function consumeAvatarDaily(discordUserId: string, avatarId: string): number | null {
  return consume(`avatar:${discordUserId}:${avatarId}`, AVATAR_DAILY_LIMIT, 1)
}

export function remainingAvatarDaily(discordUserId: string, avatarId: string): number {
  return remaining(`avatar:${discordUserId}:${avatarId}`, AVATAR_DAILY_LIMIT)
}
