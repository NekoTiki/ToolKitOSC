import { getAiProviders } from '~~/server/utils/ai'
import { getAiAccess } from '~~/server/utils/ai/access'
import { ACCOUNT_DAILY_CREDITS, PROMPT_PROFILES } from '~~/server/utils/ai/profiles'
import { remainingAccountCredits } from '~~/server/utils/ai/rateLimit'

// Lets the client build its provider/profile pickers (and grey out/hide combinations with no key
// configured on this server, or show remaining credits) without hardcoding either list or
// guessing at availability client-side.
export default defineEventHandler(async (event) => {
  const user = verifyDesktopToken(event)
  const discordId = user.discord?.id

  if (!discordId) throw createError({ statusCode: 401, statusMessage: 'No Discord identity on token' })

  // Defense in depth - the client won't normally call this since the AI button/modal is hidden
  // entirely without access (see useAiAccess.ts), but this route shouldn't leak provider/credit
  // detail to an account that isn't allowed to use the feature either way.
  const access = await getAiAccess(discordId)

  if (!access.hasAccess) throw createError({ statusCode: 403, statusMessage: "You don't have access to the AI control-suggestion feature." })

  const providers = getAiProviders().map((provider) => ({
    id: provider.id,
    label: provider.label,
    configured: provider.isConfigured()
  }))

  const profiles = Object.values(PROMPT_PROFILES).map((profile) => ({
    id: profile.id,
    label: profile.label,
    description: profile.description,
    cost: profile.creditCost
  }))

  return {
    providers,
    profiles,
    // One overall pool per account per day, shared across every provider (see rateLimit.ts) - not
    // per-provider, every profile just spends from this same number at a different rate
    // (profile.creditCost above).
    remainingCredits: await remainingAccountCredits(discordId, ACCOUNT_DAILY_CREDITS),
    dailyCredits: ACCOUNT_DAILY_CREDITS,
    // Profile is always the client's own choice (see suggest-controls.post.ts) - only the
    // provider/model is gated by this.
    canSelectModel: access.canSelectModel
  }
})
