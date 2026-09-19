import type { ControlGroup } from '@vrc-osc-toolkit/shared-ui'

import { getAiProvider } from '~~/server/utils/ai'
import { normalizeSuggestion } from '~~/server/utils/ai/normalize'
import type { AiParameterInput } from '~~/server/utils/ai/types'

const MAX_PARAMETERS = 1000
const VALID_KINDS = new Set(['Bool', 'Float', 'Int'])

interface RequestBody {
  provider: string
  avatarName: string
  parameters: AiParameterInput[]
}

// Bearer-token-gated (see verifyDesktopToken) so this never sits open as an unauthenticated proxy
// burning the Groq/Gemini keys configured on this server - the desktop client already holds a
// token from its normal Discord auth flow (see apps/client's useAuth.ts).
export default defineEventHandler(async (event): Promise<{ groups: ControlGroup[] }> => {
  verifyDesktopToken(event)

  const body = await readBody<Partial<RequestBody>>(event)

  if (!body?.provider || typeof body.provider !== 'string') {
    throw createError({ statusCode: 400, statusMessage: 'Missing provider' })
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

  try {
    const suggestion = await provider.suggestControlGroups(body.avatarName, body.parameters)
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
