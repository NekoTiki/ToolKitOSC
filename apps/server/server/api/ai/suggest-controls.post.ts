import type { ControlGroup } from '@vrc-osc-toolkit/shared-ui'

import { getAiProvider } from '~~/server/utils/ai'
import { normalizeSuggestion } from '~~/server/utils/ai/normalize'
import { ACCOUNT_DAILY_CREDITS, getPromptProfile } from '~~/server/utils/ai/profiles'
import { consumeAccountCredits, consumeAvatarDaily } from '~~/server/utils/ai/rateLimit'
import type { AiParameterInput } from '~~/server/utils/ai/types'

const MAX_PARAMETERS = 1000
const VALID_KINDS = new Set(['Bool', 'Float', 'Int'])

interface RequestBody {
  provider: string
  profile: string
  avatarId: string
  avatarName: string
  parameters: AiParameterInput[]
}

// Bearer-token-gated (see verifyDesktopToken) so this never sits open as an unauthenticated proxy
// burning the configured provider keys - the desktop client already holds a token from its normal
// Discord auth flow (see apps/client's useAuth.ts). Model choice is entirely server-side (see
// profiles.ts's resolveModel) - the client only ever picks a provider + profile, never a model.
export default defineEventHandler(async (event): Promise<{ groups: ControlGroup[] }> => {
  const user = verifyDesktopToken(event)
  const discordId = user.discord?.id

  if (!discordId) throw createError({ statusCode: 401, statusMessage: 'No Discord identity on token' })

  const body = await readBody<Partial<RequestBody>>(event)

  if (!body?.provider || typeof body.provider !== 'string') {
    throw createError({ statusCode: 400, statusMessage: 'Missing provider' })
  }

  const profile = getPromptProfile(body.profile)

  if (!profile) {
    throw createError({ statusCode: 400, statusMessage: 'Missing or unknown profile' })
  }

  if (!body.avatarId || typeof body.avatarId !== 'string') {
    throw createError({ statusCode: 400, statusMessage: 'Missing avatarId' })
  }

  if (!body.avatarName || typeof body.avatarName !== 'string') {
    throw createError({ statusCode: 400, statusMessage: 'Missing avatarName' })
  }

  if (!Array.isArray(body.parameters) || body.parameters.length === 0) {
    throw createError({ statusCode: 400, statusMessage: 'Missing parameters' })
  }

  if (body.parameters.length > MAX_PARAMETERS) {
    throw createError({ statusCode: 400, statusMessage: `Too many parameters (max ${MAX_PARAMETERS})` })
  }

  for (const param of body.parameters) {
    if (
      typeof param.name !== 'string' ||
      typeof param.address !== 'string' ||
      !VALID_KINDS.has(param.kind)
    ) {
      throw createError({ statusCode: 400, statusMessage: 'Malformed parameter entry' })
    }
  }

  const provider = getAiProvider(body.provider)

  if (!provider) {
    throw createError({ statusCode: 400, statusMessage: `Unknown provider: ${body.provider}` })
  }

  if (!provider.isConfigured()) {
    throw createError({ statusCode: 503, statusMessage: `${provider.label} is not configured on this server` })
  }

  // Two independent limits (see rateLimit.ts) - checked, and consumed, before spending a request
  // on the provider itself.
  if (consumeAvatarDaily(discordId, body.avatarId) === null) {
    throw createError({
      statusCode: 429,
      statusMessage: "You've reached today's generation limit for this avatar. Try again tomorrow."
    })
  }

  if (consumeAccountCredits(discordId, provider.id, profile.creditCost, ACCOUNT_DAILY_CREDITS) === null) {
    throw createError({
      statusCode: 429,
      statusMessage: `Not enough ${provider.label} credits left today for a ${profile.label} generation (costs ${profile.creditCost}). Try a lighter profile, a different provider, or again tomorrow.`
    })
  }

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
