import { z } from 'zod'

import { logAdminAction } from '~~/server/db/adminLog'
import { sendMessageToHost } from '~~/server/routes/host'
import { dailyCreditsFor, setDailyCredits } from '~~/server/utils/ai/access'
import { remainingAccountCredits } from '~~/server/utils/ai/rateLimit'

// null: back to the server-wide setting.
const bodySchema = z.object({ dailyCredits: z.number().int().min(0).max(10_000).nullable() })

// A lasting daily pool for this account, unlike the one-day top-up in credits.post.ts.
export default defineEventHandler(async (event) => {
  const { discordId: adminId } = await requireAdmin(event)

  const discordId = getRouterParam(event, 'discordId')

  if (!discordId) throw createError({ statusCode: 400, statusMessage: 'Missing discordId' })

  const { dailyCredits } = await readValidatedBody(event, bodySchema.parse)

  await setDailyCredits(discordId, dailyCredits)

  const daily = await dailyCreditsFor(discordId)

  sendMessageToHost(discordId, 'ai-credits-update', { remainingCredits: await remainingAccountCredits(discordId, daily), dailyCredits: daily })

  await logAdminAction(adminId, 'set-daily-credits', discordId, { dailyCredits })

  return { success: true }
})
