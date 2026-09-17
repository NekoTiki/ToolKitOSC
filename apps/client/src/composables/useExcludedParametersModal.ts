import { ref } from 'vue'

// Just the open flag - unlike usePresetModal/useLockedControlsModal there's no form/draft model
// here, this modal edits usePresets.ts's `excludedAddresses` directly, live, one toggle at a time.
const open = ref(false)

export function useExcludedParametersModal(): { open: typeof open } {
  return { open }
}
