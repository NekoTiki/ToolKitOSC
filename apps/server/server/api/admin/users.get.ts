import { listAccessEntries } from '~~/server/utils/ai/access'
import { ACCOUNT_DAILY_CREDITS } from '~~/server/utils/ai/profiles'
import { remainingAccountCredits } from '~~/server/utils/ai/rateLimit'

// Every known Discord user (not just ones already granted access - see listAccessEntries), each
// with today's remaining overall credits (one pool per account, not per provider - see
// rateLimit.ts), for the /dashboard/users table.
export default defineEventHandler(async (event) => {
  await requireAdmin(event)

  const entries = await listAccessEntries()

  const users = await Promise.all(
    entries.map(async (entry) => ({
      ...entry,
      credits: entry.hasAccess
        ? { remaining: await remainingAccountCredits(entry.discordId, ACCOUNT_DAILY_CREDITS) }
        : null
    }))
  )

  return { users }
})
