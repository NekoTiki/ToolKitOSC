import { statsQuerySchema } from '~~/server/utils/ai/schemas'
import { queryGlobalControlStats } from '~~/server/utils/controlStats'

// Fleet-wide control inventory/activation stats for the /dashboard/stats "Controls" section -
// summed across every host, the same way GET /api/admin/stats is the fleet-wide view of AI
// generations. See api/admin/users/[discordId]/stats.get.ts for the per-user equivalent.
export default defineEventHandler(async (event) => {
  await requireAdmin(event)

  const { days } = await getValidatedQuery(event, statsQuerySchema.parse)

  return queryGlobalControlStats(days)
})
