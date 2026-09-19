import { queryStats } from '~~/server/utils/ai/generationLog'
import { statsQuerySchema } from '~~/server/utils/ai/schemas'

export default defineEventHandler(async (event) => {
  await requireAdmin(event)

  const { days } = await getValidatedQuery(event, statsQuerySchema.parse)

  return queryStats(days)
})
