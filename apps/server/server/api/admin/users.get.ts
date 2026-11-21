import { hostList } from '~~/server/routes/host'
import { enabledAddonsByUser } from '~~/server/utils/addonStats'
import { listAccessEntries } from '~~/server/utils/ai/access'
import { remainingAccountCredits } from '~~/server/utils/ai/rateLimit'
import { getSettings } from '~~/server/utils/serverSettings'

// Every known Discord user (not just ones already granted access - see listAccessEntries), each
// with today's remaining overall credits (one pool per account, not per provider - see
// rateLimit.ts), for the /dashboard/users table.
export default defineEventHandler(async (event) => {
  await requireAdmin(event)

  const [entries, addons] = await Promise.all([listAccessEntries(), enabledAddonsByUser()])

  const users = await Promise.all(
    entries.map(async (entry) => ({
      ...entry,
      addons: addons.get(entry.discordId) ?? [],
      // Their app is connected right now: since when, or null.
      onlineSince: hostList.get(entry.discordId)?.connectedAt ?? null,
      credits: entry.hasAccess
        ? { remaining: await remainingAccountCredits(entry.discordId, entry.dailyCredits ?? getSettings().dailyCredits), daily: entry.dailyCredits ?? getSettings().dailyCredits }
        : null
    }))
  )

  return { users, dailyCredits: getSettings().dailyCredits }
})
