import { ref } from 'vue'

// Same pattern as useIntifaceControl.ts: the currently-playing pattern id (or 'off') for an
// 'intiface-pattern' control, kept as a module-scope singleton so every viewer's chip picker (and
// the host's own) shows the same choice, synced via 'intiface-pattern-value-update'.
export type IntifacePatternControlValue = string

// Set while a viewer's own pattern (VIEWER_PATTERN_ID) plays: who made it and when it stops by
// itself (ms since epoch; absent: it doesn't).
export interface ViewerPatternInfo {
  by?: string
  endsAt?: number
}

const controlValue = ref<Map<string, IntifacePatternControlValue>>(new Map())
const viewerPatterns = ref<Map<string, ViewerPatternInfo>>(new Map())

const lastUpdate = ref<({ controlId: string; value: IntifacePatternControlValue } & ViewerPatternInfo) | null>(null)

export function useIntifacePatternControl(): {
  controlValue: typeof controlValue
  viewerPatterns: typeof viewerPatterns
  lastUpdate: typeof lastUpdate
  setValue: (controlId: string, value: IntifacePatternControlValue, info?: ViewerPatternInfo) => void
} {
  const setValue = (controlId: string, value: IntifacePatternControlValue, info?: ViewerPatternInfo): void => {
    const next = new Map(controlValue.value)

    next.set(controlId, value)
    controlValue.value = next

    const nextInfo = new Map(viewerPatterns.value)

    if (info) nextInfo.set(controlId, info)
    else nextInfo.delete(controlId)
    viewerPatterns.value = nextInfo

    lastUpdate.value = { controlId, value, ...info }
  }

  return { controlValue, viewerPatterns, lastUpdate, setValue }
}
