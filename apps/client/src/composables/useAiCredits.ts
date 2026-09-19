import type { AiCreditsUpdateMessage } from '@vrc-osc-toolkit/shared-ui'
import { ref } from 'vue'

// Module-scope, not per-caller state - same pattern as useAuth/useOpenShock: the server pushes
// this over the host WS connection (see useWebsocketHost.ts's 'ai-credits-update' handler)
// whenever POST /api/ai/suggest-controls actually spends from an account's credit pool or an
// avatar's daily slot, independent of whether AiControlsModal.vue happens to be open right now.
// One overall pool per account per day, shared across every provider - not per-provider.
const latest = ref<AiCreditsUpdateMessage | null>(null)

export function useAiCredits(): {
  latest: typeof latest
  applyUpdate: (update: AiCreditsUpdateMessage) => void
} {
  const applyUpdate = (update: AiCreditsUpdateMessage): void => {
    latest.value = update
  }

  return { latest, applyUpdate }
}
