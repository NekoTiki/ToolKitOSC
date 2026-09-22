import { eq } from 'drizzle-orm'

import { getDb } from '~~/server/db'
import { users } from '~~/server/db/schema'
import { queryStats } from '~~/server/utils/ai/generationLog'
import { statsQuerySchema } from '~~/server/utils/ai/schemas'
import { queryUserControlStats } from '~~/server/utils/controlStats'

// One account's own slice of AI-generation and control usage stats - backs the per-user
// drill-down page linked from /dashboard/users (see GET /api/admin/attempts?discordId=... for
// that same account's recent-generations list, already filterable that way).
export default defineEventHandler(async (event) => {
  await requireAdmin(event)

  const discordId = getRouterParam(event, 'discordId')

  if (!discordId) throw createError({ statusCode: 400, statusMessage: 'Missing discordId' })

  const { days } = await getValidatedQuery(event, statsQuerySchema.parse)

  const [[user], ai, controls] = await Promise.all([
    getDb().select().from(users).where(eq(users.discordId, discordId)).limit(1),
    queryStats(days, discordId),
    queryUserControlStats(discordId, days)
  ])

  if (!user) throw createError({ statusCode: 404, statusMessage: 'User not found' })

  return { user, ai, controls }
})
