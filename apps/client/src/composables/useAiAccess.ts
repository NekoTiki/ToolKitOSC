import type { AiAccessUpdateMessage } from '@toolkitosc/shared-ui'
import { ref } from 'vue'

// Module-scope, not per-caller state - same pattern as useAiCredits.ts. Populated from two
// sources: the initial value arrives on 'auth-success' (see useWebsocketHost.ts), and it's kept
// live afterward by 'ai-access-update' pushes whenever an admin changes this account's access from
// /dashboard. Defaults closed (no access, no model-select) until either of those actually fires.
const access = ref<AiAccessUpdateMessage>({ hasAccess: false, canSelectModel: false })

export function useAiAccess(): {
  access: typeof access
  applyUpdate: (update: AiAccessUpdateMessage) => void
} {
  const applyUpdate = (update: AiAccessUpdateMessage): void => {
    access.value = update
  }

  return { access, applyUpdate }
}
