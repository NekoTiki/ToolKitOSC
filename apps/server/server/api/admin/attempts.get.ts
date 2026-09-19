import { listRecentAttempts } from '~~/server/utils/ai/generationLog'
import { recentAttemptsQuerySchema } from '~~/server/utils/ai/schemas'

// Filterable/paginated generation log for the /dashboard/stats recent-attempts table - separate
// from GET /api/admin/stats, which is purely aggregate chart data over a day window.
export default defineEventHandler(async (event) => {
  await requireAdmin(event)

  const query = await getValidatedQuery(event, recentAttemptsQuerySchema.parse)

  const attempts = await listRecentAttempts(
    {
      discordId: query.discordId,
      provider: query.provider,
      success: query.success === undefined ? undefined : query.success === 'true'
    },
    query.page
  )

  return { attempts }
})
