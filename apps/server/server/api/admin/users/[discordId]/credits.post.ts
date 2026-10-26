import { logAdminAction } from '~~/server/db/adminLog'
import { sendMessageToHost } from '~~/server/routes/host'
import { dailyCreditsFor } from '~~/server/utils/ai/access'
import { addCreditBonus, remainingAccountCredits } from '~~/server/utils/ai/rateLimit'
import { creditBonusBodySchema } from '~~/server/utils/ai/schemas'

// Admin "increase credits for today" top-up (see rateLimit.ts's addCreditBonus) - additive on top
// of the account's one overall daily pool, only for today.
export default defineEventHandler(async (event) => {
  const { discordId: adminId } = await requireAdmin(event)

  const discordId = getRouterParam(event, 'discordId')

  if (!discordId) throw createError({ statusCode: 400, statusMessage: 'Missing discordId' })

  const { amount } = await readValidatedBody(event, creditBonusBodySchema.parse)

  await addCreditBonus(discordId, amount)

  const dailyCredits = await dailyCreditsFor(discordId)
  const remaining = await remainingAccountCredits(discordId, dailyCredits)

  // Reuses the same 'ai-credits-update' push the generation endpoint sends, so the AI modal's
  // displayed balance updates live if this account has an open host connection right now.
  sendMessageToHost(discordId, 'ai-credits-update', {
    remainingCredits: remaining,
    dailyCredits
  })

  await logAdminAction(adminId, 'add-credits', discordId, { amount })

  return { remaining }
})
