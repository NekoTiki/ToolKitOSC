// Response shapes of GET /api/admin/stats and /api/admin/control-stats (and the `ai`/`controls`
// halves of a user's /api/admin/users/:id/stats), shared by the Overview and user pages.
export interface StatsResult {
  daily: { day: string; success: number; failure: number }[]
  byProvider: { provider: string | null; providerLabel: string | null; success: number; failure: number; avgDurationMs: number | null }[]
  byProfile: { profile: string | null; count: number }[]
  totalSuccess: number
  totalFailure: number
}

export interface ControlStatsResult {
  inventory: { type: string; count: number }[]
  totalControls: number
  activationByType: { type: string; count: number }[]
  activationDaily: { day: string; count: number }[]
  totalActivations: number
}

// The date ranges every dashboard usage view offers (the API accepts 1-90 days).
export const DAY_RANGES = [7, 14, 30, 90].map((value) => ({ value, label: `${value} days` }))
