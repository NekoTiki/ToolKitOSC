// Icons for the "recent generations" log shared by dashboard/stats.vue and
// dashboard/user/[discordId].vue - purely a presentation concern, so kept here rather than
// alongside the server's FailureReason type (see generationLog.ts).
const FAILURE_REASON_ICONS: Record<string, string> = {
  'access-denied': 'i-lucide-lock',
  'credit-limit': 'i-lucide-credit-card',
  'provider-error': 'i-lucide-server-crash'
}

export function useAiLogIcons(): {
  successIcon: string
  failureIcon: (reason: string | null) => string
  refundedIcon: string
  providerIcon: string
  profileIcon: string
} {
  return {
    successIcon: 'i-lucide-check',
    // Falls back to a generic error icon for a reason this map doesn't know about (or none at
    // all) rather than rendering no icon - keeps every failed badge visually consistent.
    failureIcon: (reason) => (reason && FAILURE_REASON_ICONS[reason]) || 'i-lucide-circle-x',
    refundedIcon: 'i-lucide-rotate-ccw',
    providerIcon: 'i-lucide-cpu',
    profileIcon: 'i-lucide-gauge'
  }
}
