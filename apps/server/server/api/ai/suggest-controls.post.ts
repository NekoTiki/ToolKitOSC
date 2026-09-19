import type { ControlGroup } from '@vrc-osc-toolkit/shared-ui'

import { sendMessageToHost } from '~~/server/routes/host'
import { getAiProvider } from '~~/server/utils/ai'
import { normalizeSuggestion } from '~~/server/utils/ai/normalize'
import { ACCOUNT_DAILY_CREDITS, PROMPT_PROFILES } from '~~/server/utils/ai/profiles'
import { AVATAR_DAILY_LIMIT, consumeAccountCredits, consumeAvatarDaily } from '~~/server/utils/ai/rateLimit'
import { suggestControlsBodySchema } from '~~/server/utils/ai/schemas'

// Bearer-token-gated (see verifyDesktopToken) so this never sits open as an unauthenticated proxy
// burning the configured provider keys - the desktop client already holds a token from its normal
// Discord auth flow (see apps/client's useAuth.ts). Model choice is entirely server-side (see
// profiles.ts's resolveModel) - the client only ever picks a provider + profile, never a model.
export default defineEventHandler(async (event): Promise<{ groups: ControlGroup[] }> => {
  const user = verifyDesktopToken(event)
  const discordId = user.discord?.id

  if (!discordId) throw createError({ statusCode: 401, statusMessage: 'No Discord identity on token' })

  // Shape/type validation (non-empty strings, a known profile id, well-formed parameters) is
  // entirely zod's job now - readValidatedBody turns a thrown ZodError into a 400 on its own, so
  // none of that needs hand-written checks here anymore. What's left below is validation zod
  // can't do: does this provider actually exist/is it configured, and rate limits.
  const body = await readValidatedBody(event, suggestControlsBodySchema.parse)
  const profile = PROMPT_PROFILES[body.profile]

  const provider = getAiProvider(body.provider)

  if (!provider) {
    throw createError({ statusCode: 400, statusMessage: `Unknown provider: ${body.provider}` })
  }

  if (!provider.isConfigured()) {
    throw createError({ statusCode: 503, statusMessage: `${provider.label} is not configured on this server` })
  }

  // Two independent limits (see rateLimit.ts) - checked, and consumed, before spending a request
  // on the provider itself. Both consume*() calls return the fresh remaining count on success, so
  // there's no need for a separate remaining*() lookup just to build the WS push below.
  const avatarRemaining = consumeAvatarDaily(discordId, body.avatarId)

  if (avatarRemaining === null) {
    throw createError({
      statusCode: 429,
      statusMessage: "You've reached today's generation limit for this avatar. Try again tomorrow."
    })
  }

  const creditsRemaining = consumeAccountCredits(discordId, provider.id, profile.creditCost, ACCOUNT_DAILY_CREDITS)

  if (creditsRemaining === null) {
    throw createError({
      statusCode: 429,
      statusMessage: `Not enough ${provider.label} credits left today for a ${profile.label} generation (costs ${profile.creditCost}). Try a lighter profile, a different provider, or again tomorrow.`
    })
  }

  // Pushed the moment both limits are actually spent, not left for the client to notice by
  // re-polling GET /api/ai/providers - regardless of whether the generation below goes on to
  // succeed, since the charge already happened. A no-op if this account has no live host
  // connection right now (sendMessageToHost already handles that - see routes/host.ts).
  sendMessageToHost(discordId, 'ai-credits-update', {
    provider: provider.id,
    remainingCredits: creditsRemaining,
    dailyCredits: ACCOUNT_DAILY_CREDITS,
    avatarId: body.avatarId,
    avatarLimit: { max: AVATAR_DAILY_LIMIT, remaining: avatarRemaining }
  })

  try {
    const suggestion = await provider.suggestControlGroups(body.avatarName, body.parameters, profile)
    const groups = normalizeSuggestion(suggestion, body.parameters)

    return { groups }
  } catch (error) {
    // Full detail (provider error text, which can include the raw upstream response body) stays
    // server-side only - useful for debugging later, not something to hand back to the client,
    // which gets a generic message instead.
    console.error(`[ai] ${provider.id} request failed for avatar "${body.avatarName}":`, error)

    throw createError({ statusCode: 502, statusMessage: 'AI generation failed. Please try again later.' })
  }
})
