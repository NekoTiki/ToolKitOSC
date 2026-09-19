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

// provider is intentionally a plain non-empty string, not a matching enum here - getAiProvider()
// (called after validation) already does that lookup against whatever providers actually exist,
// dynamically, and returns a clean 400/503 itself; duplicating that as a second static list here
// would just be one more place for the two to drift apart.
export const suggestControlsBodySchema = z.object({
  provider: z.string().min(1),
  profile: z.enum(PROFILE_IDS),
  avatarId: z.string().min(1),
  avatarName: z.string().min(1),
  parameters: z.array(parameterSchema).min(1).max(MAX_PARAMETERS)
})

export const optionsQuerySchema = z.object({
  // Optional - GET /api/ai/providers is still useful without one (provider/profile listing,
  // credit balances), it just can't also report the per-avatar limit until an avatar is known.
  avatarId: z.string().min(1).optional()
})
