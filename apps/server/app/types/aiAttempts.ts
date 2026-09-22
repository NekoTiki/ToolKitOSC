// Shared between RecentGenerationsCard.vue (which owns fetching/live-updating these) and any page
// embedding it that also wants to react to the same live events for its own purposes (see
// dashboard/stats.vue's own chart aggregates).
export interface Attempt {
  id: number
  requestId?: string
  discordId: string
  displayName: string | null
  avatarName: string | null
  provider: string | null
  providerLabel: string | null
  profile: string | null
  profileLabel: string | null
  model: string | null
  success: boolean
  failureReason: string | null
  failureReasonLabel: string | null
  errorMessage: string | null
  refunded: boolean
  durationMs: number | null
  createdAt: string
}

// A generation still in flight (see the server's liveGenerations.ts) - no `id`/`success`/duration
// yet, since those only exist once ai_generation_log actually has a row for it.
export interface LiveAttempt {
  requestId: string
  discordId: string
  displayName: string | null
  avatarName: string
  provider: string
  providerLabel: string
  profile: string
  profileLabel: string
  model: string
  startedAt: string
}
