import { z } from 'zod'

import type { PromptProfileId } from './profiles'
import { PROMPT_PROFILES } from './profiles'

const MAX_PARAMETERS = 1000

// Runtime values, not a hand-duplicated literal list - stays correct if a profile is ever
// added/removed from profiles.ts without needing a matching edit here. The cast is safe: every
// key of PROMPT_PROFILES is, by construction, a PromptProfileId.
const PROFILE_IDS = Object.keys(PROMPT_PROFILES) as [PromptProfileId, ...PromptProfileId[]]

const parameterSchema = z.object({
  name: z.string().min(1),
  address: z.string().min(1),
  kind: z.enum(['Bool', 'Float', 'Int'])
})

// Shared leaf validators used across every control-suggestion shape in normalize.ts, not just
// step-enum's - a provider's raw JSON is untrusted the same way regardless of which control type
// it's for, so every name/address the model hands back gets the same trim-and-require-non-empty
// treatment before it's used, rather than each control type re-implementing its own truthy check.
export const aiControlNameSchema = z.string().trim().min(1)
export const aiAddressSchema = z.string().trim().min(1)

// A single 'step-enum' option as a provider actually returns it - validated in normalize.ts before
// it's trusted into a real StepEnumControl. `value` is clamped to 0-255: VRChat Int parameters are
// a single byte, so anything outside that range could never actually be reached from OSC and would
// just render as a permanently-dead option rather than a working one.
export const stepEnumOptionSchema = z.object({
  name: aiControlNameSchema,
  value: z.number().int().min(0).max(255),
  icon: z.string().optional()
})

// provider is intentionally a plain non-empty string, not a matching enum here - getAiProvider()
// (called after validation) already does that lookup against whatever providers actually exist,
// dynamically, and returns a clean 400/503 itself; duplicating that as a second static list here
// would just be one more place for the two to drift apart.
// Both optional, but for different reasons: `provider` is ignored entirely (regardless of what's
// sent) unless this account has model-select permission (see utils/ai/access.ts) - that's the
// actual enforcement point for that permission, not just a client convenience. `profile` is always
// honored when sent - every account picks its own profile, permission or not - optional here only
// as a safe fallback to resolveDefaultProfile() if it's ever omitted.
export const suggestControlsBodySchema = z.object({
  provider: z.string().min(1).optional(),
  profile: z.enum(PROFILE_IDS).optional(),
  avatarId: z.string().min(1),
  avatarName: z.string().min(1),
  parameters: z.array(parameterSchema).min(1).max(MAX_PARAMETERS)
})

// Admin routes (server/api/admin/*) - all gated by requireAdmin, but still validated the same way.
export const grantAccessBodySchema = z.object({
  discordId: z.string().min(1),
  canSelectModel: z.boolean().optional().default(false),
  note: z.string().max(500).optional()
})

export const setCanSelectModelBodySchema = z.object({
  canSelectModel: z.boolean()
})

// One overall pool per account per day (see rateLimit.ts) - not per-provider.
export const creditBonusBodySchema = z.object({
  amount: z.number().int().min(1).max(1000)
})

export const searchUsersQuerySchema = z.object({
  q: z.string().min(1).max(100)
})

export const statsQuerySchema = z.object({
  days: z.coerce.number().int().min(1).max(90).optional().default(14)
})

export const recentAttemptsQuerySchema = z.object({
  discordId: z.string().min(1).optional(),
  provider: z.string().min(1).optional(),
  success: z.enum(['true', 'false']).optional(),
  page: z.coerce.number().int().min(1).optional().default(1)
})
