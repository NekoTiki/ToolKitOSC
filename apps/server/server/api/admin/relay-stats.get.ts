import { statsQuerySchema } from '~~/server/utils/ai/schemas'
import { queryRelayStats } from '~~/server/utils/relayStats'

export default defineEventHandler(async (event) => {
  await requireAdmin(event)

  const { days } = await getValidatedQuery(event, statsQuerySchema.parse)

  return { days: await queryRelayStats(days) }
})
