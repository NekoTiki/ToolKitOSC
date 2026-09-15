import type { Ref } from 'vue'
import { ref, watch } from 'vue'

// Memoized per group id so every call for the same group returns the same ref (and only ever
// registers one localStorage-syncing watcher for it), rather than resetting to the stored value
// on every re-render.
const openRefs = new Map<string, Ref<boolean>>()

export function useControlGroupOpen(groupId: string): Ref<boolean> {
  const existing = openRefs.get(groupId)

  if (existing) return existing

  const storageKey = `controlGroupOpen_${groupId}`
  const open = ref(import.meta.client ? localStorage.getItem(storageKey) !== 'false' : true)

  watch(open, (value) => {
    if (import.meta.client) localStorage.setItem(storageKey, String(value))
  })

  openRefs.set(groupId, open)

  return open
}
