import type { AiSuggestionResult } from './types'

// Thrown by a provider's suggestControlGroups when the upstream rejected the request before any
// generation actually ran (rate limit, payload too large, bad/expired key, out of quota) - as
// opposed to a request that reached the model and then failed for some other reason (e.g. output
// that doesn't validate against the strict json_schema), where real inference already happened.
// suggest-controls.post.ts refunds the spent credits when it catches one of these instead of a
// plain Error, since nothing was actually delivered for that spend.
export class AiRefundableError extends Error {
  constructor(message: string, options?: { cause?: unknown }) {
    super(message, options)
    this.name = 'AiRefundableError'
  }
}

interface OpenAiStyleErrorBody {
  error?: { message?: string; type?: string; code?: string; failed_generation?: string }
}

function parseOpenAiErrorBody(rawBody: string): OpenAiStyleErrorBody | undefined {
  try {
    return JSON.parse(rawBody) as OpenAiStyleErrorBody
  } catch {
    return undefined
  }
}

// Groq's (and possibly other OpenAI-compatible providers using the same structured-output
// validator's) own strict json_schema validator sometimes rejects a response that's still
// perfectly usable - a stray duplicate JSON key from the model is a known case: JSON.parse
// tolerates it (silently keeps the last occurrence, per the JSON spec), a schema validator
// doesn't. The provider includes the raw generation it rejected in the error body specifically for
// this kind of recovery, and our own normalize.ts is already more lenient than the provider's
// validator (it drops/repairs malformed controls rather than failing outright), so it's worth
// trying to use before burning a whole retry over what's often a cosmetic decoding glitch.
// Conservative: only attempts recovery for the specific error code that means "the JSON just
// didn't validate," and only returns something if it actually parses into the right rough shape.
export function tryRecoverFailedGeneration(rawBody: string): AiSuggestionResult | null {
  const parsed = parseOpenAiErrorBody(rawBody)

  if (parsed?.error?.code !== 'json_validate_failed') return null

  const raw = parsed.error.failed_generation

  if (!raw) return null

  try {
    const recovered = JSON.parse(raw) as AiSuggestionResult

    return Array.isArray(recovered?.groups) ? recovered : null
  } catch {
    return null
  }
}

// Shared by every OpenAI-compatible provider (Groq, Mistral, Cloudflare Workers AI, OpenRouter all
// use this same {error:{message,type,code}} response shape on failure) - conservative by design:
// only status/code combinations we're confident mean "the request was rejected before the model
// ever ran" are refundable. Everything else (including a strict-schema validation failure on real
// generated output - that means the model DID run) throws a normal, non-refundable error instead.
// Extend these sets, not the call sites, if a new non-billable case turns up for one of these
// providers - they all route through here.
const REFUNDABLE_HTTP_STATUS = new Set([401, 402, 403, 404, 408, 413, 429])
const REFUNDABLE_ERROR_CODES = new Set([
  'rate_limit_exceeded',
  'context_length_exceeded',
  'invalid_api_key',
  'insufficient_quota',
  'payment_required',
  'model_not_found'
])
const REFUNDABLE_ERROR_TYPES = new Set(['tokens', 'rate_limit_exceeded', 'insufficient_quota', 'payment_required_error', 'not_found_error'])

export function throwOpenAiCompatibleError(providerLabel: string, status: number, rawBody: string): never {
  const message = `${providerLabel} request failed (${status}): ${rawBody}`
  const parsed = parseOpenAiErrorBody(rawBody)
  const code = parsed?.error?.code
  const type = parsed?.error?.type
  const refundable =
    REFUNDABLE_HTTP_STATUS.has(status) ||
    (!!code && REFUNDABLE_ERROR_CODES.has(code)) ||
    (!!type && REFUNDABLE_ERROR_TYPES.has(type))

  if (refundable) throw new AiRefundableError(message)

  throw new Error(message)
}

interface GeminiErrorBody {
  error?: { message?: string; status?: string; code?: number }
}

// Gemini's error shape is Google's own API convention ({error:{code,message,status}}, where
// `status` is an uppercase enum string), not the OpenAI one - same conservative split as above.
const REFUNDABLE_GEMINI_STATUS = new Set(['RESOURCE_EXHAUSTED', 'UNAUTHENTICATED', 'PERMISSION_DENIED', 'INVALID_ARGUMENT'])

export function throwGeminiError(status: number, rawBody: string): never {
  const message = `Gemini request failed (${status}): ${rawBody}`
  let parsed: GeminiErrorBody | undefined

  try {
    parsed = JSON.parse(rawBody) as GeminiErrorBody
  } catch {
    // Not JSON - falls through to the status-only check below.
  }

  const geminiStatus = parsed?.error?.status
  const refundable =
    status === 401 ||
    status === 403 ||
    status === 429 ||
    (!!geminiStatus && REFUNDABLE_GEMINI_STATUS.has(geminiStatus))

  if (refundable) throw new AiRefundableError(message)

  throw new Error(message)
}
