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

// The app's add-ons, as its Add-ons page names them. `devices` names what the count counts; none
// for add-ons without devices. An id missing here (an app newer than this server) shows as itself.
export const ADDONS: Record<string, { name: string; icon: string; devices?: string }> = {
  openshock: { name: 'OpenShock', icon: 'i-lucide-zap', devices: 'shockers' },
  intiface: { name: 'Intiface toys', icon: 'i-lucide-vibrate', devices: 'toys' },
  haptics: { name: 'Avatar haptics', icon: 'i-lucide-hand' },
  leash: { name: 'Avatar leash', icon: 'i-lucide-link' },
  bhaptics: { name: 'bHaptics', icon: 'i-lucide-shirt', devices: 'devices' },
  owo: { name: 'OWO', icon: 'i-lucide-activity', devices: 'suits' },
  boards: { name: 'ESP32 boards', icon: 'i-lucide-cpu', devices: 'boards' },
  overlay: { name: 'VR overlay', icon: 'i-lucide-glasses' },
  mcp: { name: 'AI assistants', icon: 'i-lucide-bot' }
}

export const addonInfo = (id: string): { name: string; icon: string; devices?: string } => ADDONS[id] ?? { name: id, icon: 'i-lucide-puzzle' }

// One add-on in a user's /api/admin/users/:id/stats `addons`.
export interface AddonUsage {
  addonId: string
  enabled: boolean
  devices: number | null
}
