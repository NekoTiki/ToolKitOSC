import { listAccessEntries } from '~~/server/utils/ai/access'
import { remainingAccountCredits } from '~~/server/utils/ai/rateLimit'
import { getSettings } from '~~/server/utils/serverSettings'

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
        ? { remaining: await remainingAccountCredits(entry.discordId, entry.dailyCredits ?? getSettings().dailyCredits), daily: entry.dailyCredits ?? getSettings().dailyCredits }
        : null
    }))
  )

  return { users, dailyCredits: getSettings().dailyCredits }
})
