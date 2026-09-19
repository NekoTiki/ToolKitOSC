import type { ControlGroup } from '@vrc-osc-toolkit/shared-ui'

import { upsertUserFromAuth } from '~~/server/db/users'
import { sendMessageToHost } from '~~/server/routes/host'
import { getAiProvider, resolveDefaultProvider } from '~~/server/utils/ai'
import { getAiAccess } from '~~/server/utils/ai/access'
import type { FailureReason } from '~~/server/utils/ai/generationLog'
import { logGenerationAttempt } from '~~/server/utils/ai/generationLog'
import { recordProviderFailure, recordProviderSuccess } from '~~/server/utils/ai/loadBalancer'
import { normalizeSuggestion } from '~~/server/utils/ai/normalize'
import { ACCOUNT_DAILY_CREDITS, PROMPT_PROFILES, resolveDefaultProfile } from '~~/server/utils/ai/profiles'
import { AiRefundableError } from '~~/server/utils/ai/providerError'
import { consumeAccountCredits, refundAccountCredits } from '~~/server/utils/ai/rateLimit'
import { suggestControlsBodySchema } from '~~/server/utils/ai/schemas'

// Bearer-token-gated (see verifyDesktopToken) so this never sits open as an unauthenticated proxy
// burning the configured provider keys - the desktop client already holds a token from its normal
// Discord auth flow (see apps/client's useAuth.ts). Model choice is entirely server-side (see
// profiles.ts's resolveModel) - the client only ever picks a provider + profile, and only when
// it's actually allowed to (see the canSelectModel check below).
export default defineEventHandler(async (event): Promise<{ groups: ControlGroup[] }> => {
  const user = verifyDesktopToken(event)
  const discordId = user.discord?.id

  if (!discordId) throw createError({ statusCode: 401, statusMessage: 'No Discord identity on token' })

  // So credit/log rows always have a valid FK target, even for a desktop client that calls this
  // route without ever having opened a host WS connection (host.ts does the same upsert on its
  // own auth path).
  await upsertUserFromAuth(user)

  // Shape/type validation (non-empty strings, a known profile id, well-formed parameters) is
  // entirely zod's job now - readValidatedBody turns a thrown ZodError into a 400 on its own, so
  // none of that needs hand-written checks here anymore. What's left below is validation zod
  // can't do: access control, does the provider actually exist/is it configured, and rate limits.
  const body = await readValidatedBody(event, suggestControlsBodySchema.parse)

  const access = await getAiAccess(discordId)

  if (!access.hasAccess) {
    await logGenerationAttempt({
      discordId,
      avatarId: body.avatarId,
      avatarName: body.avatarName,
      success: false,
      failureReason: 'access-denied'
    })

    throw createError({
      statusCode: 403,
      statusMessage: "You don't have access to the AI control-suggestion feature yet. Ask an admin for access."
    })
  }

  // Only the provider (the actual model) is gated by permission - a real security boundary, not
  // just hidden UI: whatever the client sends is ignored entirely unless this account actually has
  // model-select permission, it can't just start sending one to bypass the server's choice. Profile
  // is always the client's choice, permission or not - it only changes prompt behavior/credit cost,
  // not which underlying model/provider answers the request.
  const provider = access.canSelectModel && body.provider ? getAiProvider(body.provider) : resolveDefaultProvider()
  const profile = body.profile ? PROMPT_PROFILES[body.profile] : resolveDefaultProfile()

  const logFailure = (
    failureReason: FailureReason,
    statusMessage: string,
    statusCode: number,
    model?: string
  ): never => {
    void logGenerationAttempt({
      discordId,
      avatarId: body.avatarId,
      avatarName: body.avatarName,
      provider: provider?.id,
      profile: profile.id,
      model,
      success: false,
      failureReason
    })

    throw createError({ statusCode, statusMessage })
  }

  if (!provider) {
    return logFailure('provider-error', 'No AI provider is configured on this server', 503)
  }

  if (!provider.isConfigured()) {
    return logFailure('provider-error', `${provider.label} is not configured on this server`, 503)
  }

  // Known before the request is even sent (see AiProvider.resolveModelForProfile) - recorded on
  // every attempt below, success or failure, so the admin stats dashboard can report a real
  // per-model success rate instead of only per-provider.
  const model = provider.resolveModelForProfile(profile.id)

  // Checked, and consumed, before spending a request on the provider itself - consumeAccountCredits
  // returns the fresh remaining count on success, so there's no need for a separate
  // remainingAccountCredits lookup just to build the WS push below.
  const creditsRemaining = await consumeAccountCredits(discordId, profile.creditCost, ACCOUNT_DAILY_CREDITS)

  if (creditsRemaining === null) {
    return logFailure(
      'credit-limit',
      `Not enough credits left today for a ${profile.label} generation (costs ${profile.creditCost}). Try a lighter profile or again tomorrow.`,
      429,
      model
    )
  }

  // Pushed the moment the credit pool is actually spent, not left for the client to notice by
  // re-polling GET /api/ai/providers - regardless of whether the generation below goes on to
  // succeed, since the charge already happened. A no-op if this account has no live host
  // connection right now (sendMessageToHost already handles that - see routes/host.ts).
  sendMessageToHost(discordId, 'ai-credits-update', {
    remainingCredits: creditsRemaining,
    dailyCredits: ACCOUNT_DAILY_CREDITS
  })

  const startedAt = Date.now()

  try {
    const suggestion = await provider.suggestControlGroups(body.avatarName, body.parameters, profile)
    const groups = normalizeSuggestion(suggestion, body.parameters)

    recordProviderSuccess(provider.id)

    void logGenerationAttempt({
      discordId,
      avatarId: body.avatarId,
      avatarName: body.avatarName,
      provider: provider.id,
      profile: profile.id,
      model,
      success: true,
      creditCost: profile.creditCost,
      durationMs: Date.now() - startedAt
    })

    return { groups }
  } catch (error) {
    // Full detail (provider error text, which can include the raw upstream response body) stays
    // server-side only - useful for debugging later, not something to hand back to the client,
    // which gets a generic message instead.
    console.error(`[ai] ${provider.id} request failed for avatar "${body.avatarName}":`, error)

    // Feeds the load balancer's circuit breaker (see loadBalancer.ts) - repeated failures take
    // this provider out of resolveDefaultProvider()'s rotation for a cooldown period, regardless
    // of whether this particular request had it auto-selected or explicitly picked.
    recordProviderFailure(provider.id)

    // A provider throws AiRefundableError specifically for failures that mean the request was
    // rejected before any generation actually ran (rate limit, payload too large, bad key, out of
    // quota - see providerError.ts) - nothing was delivered for that spend, so give it back. A
    // second WS push here is deliberate, not a dedupe of the one above: the first told the client
    // what it was charged, this one tells it what it got back.
    const refunded = error instanceof AiRefundableError

    if (refunded) {
      const remainingAfterRefund = await refundAccountCredits(discordId, profile.creditCost, ACCOUNT_DAILY_CREDITS)

      sendMessageToHost(discordId, 'ai-credits-update', {
        remainingCredits: remainingAfterRefund,
        dailyCredits: ACCOUNT_DAILY_CREDITS
      })
    }

    void logGenerationAttempt({
      discordId,
      avatarId: body.avatarId,
      avatarName: body.avatarName,
      provider: provider.id,
      profile: profile.id,
      model,
      success: false,
      failureReason: 'provider-error',
      errorMessage: error instanceof Error ? error.message : String(error),
      creditCost: profile.creditCost,
      refunded,
      durationMs: Date.now() - startedAt
    })

    throw createError({
      statusCode: 502,
      statusMessage: refunded
        ? "AI generation failed before it started - you weren't charged for this attempt. Please try again later."
        : 'AI generation failed. Please try again later.'
    })
  }
})
