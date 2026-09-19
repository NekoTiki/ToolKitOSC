import { getAiProviders } from '~~/server/utils/ai'
import { ACCOUNT_DAILY_CREDITS, PROMPT_PROFILES } from '~~/server/utils/ai/profiles'
import { remainingAccountCredits } from '~~/server/utils/ai/rateLimit'

// Lets the client build its provider/profile pickers (and grey out/hide combinations with no key
// configured on this server, or show remaining credits) without hardcoding either list or
// guessing at availability client-side.
export default defineEventHandler((event) => {
  const user = verifyDesktopToken(event)
  const discordId = user.discord?.id

  const providers = getAiProviders().map((provider) => ({
    id: provider.id,
    label: provider.label,
    configured: provider.isConfigured(),
    // Credits are a per-(account, provider) pool, not per-profile (see rateLimit.ts) - every
    // profile just spends from this same number at a different rate (profile.creditCost below).
    // 0 if there's no Discord id to key a lookup by, which verifyDesktopToken would already have
    // rejected before this route's own logic runs in practice, but keeps the mapping honest.
    remainingCredits: discordId ? remainingAccountCredits(discordId, provider.id, ACCOUNT_DAILY_CREDITS) : 0
  }))

  const profiles = Object.values(PROMPT_PROFILES).map((profile) => ({
    id: profile.id,
    label: profile.label,
    description: profile.description,
    cost: profile.creditCost
  }))

  return { providers, profiles, dailyCredits: ACCOUNT_DAILY_CREDITS }
})
