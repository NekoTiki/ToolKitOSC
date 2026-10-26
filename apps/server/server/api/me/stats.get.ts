import { statsQuerySchema } from '~~/server/utils/ai/schemas'
import { queryUserControlStats } from '~~/server/utils/controlStats'

// The host's own control stats (what viewers used, per day and by type), for the app's Activity
// page. The same numbers admins see on the host's dashboard page.
export default defineEventHandler(async (event) => {
  const user = await verifyDesktopToken(event)

  if (!user.discord?.id) throw createError({ statusCode: 401, statusMessage: 'No Discord identity on token' })

  const { days } = await getValidatedQuery(event, statsQuerySchema.parse)

  return queryUserControlStats(user.discord.id, days)
})
