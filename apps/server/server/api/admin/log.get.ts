import { z } from 'zod'

import { listAdminLog } from '~~/server/db/adminLog'

const querySchema = z.object({ discordId: z.string().optional(), limit: z.coerce.number().int().min(1).max(500).default(100) })

export default defineEventHandler(async (event) => {
  await requireAdmin(event)

  const { discordId, limit } = await getValidatedQuery(event, querySchema.parse)

  return { entries: await listAdminLog(discordId, limit) }
})
