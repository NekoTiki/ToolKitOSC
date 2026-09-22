import { notifyAdmins } from '~~/server/routes/admin/ws'
import { profileLabel, providerLabel } from '~~/server/utils/ai/generationLog'

// In-memory only, one process - a generation still in flight has no row in ai_generation_log yet
// (see generationLog.ts's logGenerationAttempt, only ever called once the provider call settles),
// so this is the only place "what's currently running" exists at all. Tracked here (not just
// pushed as a one-off WS event) so a dashboard that connects/reconnects *during* a generation can
// still recover it via GET /api/admin/live-attempts (listLiveGenerations below) instead of only
// finding out once the next one starts.
export interface LiveGeneration {
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

// What a caller actually knows at generation-start time - startLiveGeneration below fills in the
// human-readable labels itself (same helpers generationLog.ts uses for a finished row), so a
// caller can't forget to and the in-progress row never shows a raw id a finished one wouldn't.
export type LiveGenerationInput = Omit<LiveGeneration, 'providerLabel' | 'profileLabel'>

const liveGenerations = new Map<string, LiveGeneration>()

// Called once, right as a generation actually starts (see suggest-controls.post.ts's
// runGeneration) - bundles tracking it and broadcasting it together so a caller can't do one
// without the other.
export function startLiveGeneration(entry: LiveGenerationInput): void {
  const enriched: LiveGeneration = {
    ...entry,
    providerLabel: providerLabel(entry.provider) ?? entry.provider,
    profileLabel: profileLabel(entry.profile) ?? entry.profile
  }

  liveGenerations.set(entry.requestId, enriched)
  notifyAdmins({ type: 'generation-started', attempt: enriched })
}

// No matching WS broadcast here on purpose - logGenerationAttempt's own 'generation-logged' push
// (which now carries this same requestId) is what tells a connected dashboard to drop its live
// placeholder, right alongside the real finished row replacing it. This just stops offering the
// entry to a dashboard that connects *after* the generation has already finished.
export function endLiveGeneration(requestId: string): void {
  liveGenerations.delete(requestId)
}

export function listLiveGenerations(): LiveGeneration[] {
  return Array.from(liveGenerations.values())
}
