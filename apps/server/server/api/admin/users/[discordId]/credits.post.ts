import { sendMessageToHost } from '~~/server/routes/host'
import { ACCOUNT_DAILY_CREDITS } from '~~/server/utils/ai/profiles'
import { addCreditBonus, remainingAccountCredits } from '~~/server/utils/ai/rateLimit'
import { creditBonusBodySchema } from '~~/server/utils/ai/schemas'

// Admin "increase credits for today" top-up (see rateLimit.ts's addCreditBonus) - additive on top
// of the account's one overall daily pool, only for today.
export default defineEventHandler(async (event) => {
  await requireAdmin(event)

  const discordId = getRouterParam(event, 'discordId')

  if (!discordId) throw createError({ statusCode: 400, statusMessage: 'Missing discordId' })

  const { amount } = await readValidatedBody(event, creditBonusBodySchema.parse)

  await addCreditBonus(discordId, amount)

  const remaining = await remainingAccountCredits(discordId, ACCOUNT_DAILY_CREDITS)

  // Reuses the same 'ai-credits-update' push the generation endpoint sends, so the AI modal's
  // displayed balance updates live if this account has an open host connection right now.
  sendMessageToHost(discordId, 'ai-credits-update', {
    remainingCredits: remaining,
    dailyCredits: ACCOUNT_DAILY_CREDITS
  })

  return { remaining }
})
