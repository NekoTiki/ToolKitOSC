import { reactive } from 'vue'

const STORAGE_PREFIX = 'controlGroupOpen_'

// Module-scope, not per-ControlGroup-instance state - previously each ControlGroup.vue held its
// own local `open` ref, read from/written to localStorage only by itself. That's fine for a user
// clicking one group's own header, but ManageGroupsModal.vue's bulk collapse/expand needs to
// change an *other*, already-mounted group's open state from outside it - poking localStorage
// directly from there wouldn't do that (a mounted component's local ref doesn't react to an
// external storage write), so this promotes it to shared reactive state instead. Lazily
// initialized per group id (defaults to expanded/true) the first time it's actually read, same
// default `!== 'false'` check the old per-instance version used.
const state = reactive<Record<string, boolean>>({})

function readStored(groupId: string): boolean {
  return localStorage.getItem(`${STORAGE_PREFIX}${groupId}`) !== 'false'
}

export function useControlGroupOpenState(): {
  isOpen: (groupId: string) => boolean
  setOpen: (groupId: string, open: boolean) => void
} {
  const isOpen = (groupId: string): boolean => {
    if (!(groupId in state)) state[groupId] = readStored(groupId)

    return state[groupId]
  }

  const setOpen = (groupId: string, open: boolean): void => {
    state[groupId] = open
    localStorage.setItem(`${STORAGE_PREFIX}${groupId}`, String(open))
  }

  return { isOpen, setOpen }
}
