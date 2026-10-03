import { useAvatarDetails } from '@renderer/composables/useAvatarDetails'
import { getUUID } from '@renderer/utils/uuid'
import type { ControlLimits } from '@toolkitosc/shared-ui'
import { computed, ref, watch } from 'vue'

export type LockedControlGroup = {
  id: string
  name: string
  lockedControls: Record<string, boolean>
  // Per-control limits for viewers, keyed by control id. Optional: profiles saved before limits
  // existed load as "no limits".
  limits?: Record<string, ControlLimits>
}

// What sessionStorage holds for an avatar whose last pick this session was "No profile" - kept
// apart from no entry at all, which means "not picked yet this session, use the default".
const NO_PROFILE = 'none'

const currentLockedControlsGroup = ref<LockedControlGroup['id'] | null>(null)
// The profile that turns on the first time the avatar loads in a session. Per avatar, kept across
// restarts.
const defaultLockedControlsGroup = ref<LockedControlGroup['id'] | null>(null)
const lockedControlGroups = ref<LockedControlGroup[]>([])
const lockedControls = computed<LockedControlGroup['lockedControls']>(
  () =>
    lockedControlGroups.value.find((g) => g.id === currentLockedControlsGroup.value)
      ?.lockedControls ?? {}
)
const controlLimits = computed<NonNullable<LockedControlGroup['limits']>>(
  () => lockedControlGroups.value.find((g) => g.id === currentLockedControlsGroup.value)?.limits ?? {}
)

// "3 locked · 2 limited", counted against the controls that exist now (`controlIds`), so entries
// left behind by deleted controls don't inflate it. A locked control doesn't count as limited.
export const profileCounts = (
  profile: Pick<LockedControlGroup, 'lockedControls' | 'limits'>,
  controlIds: Set<string>
): { locked: number; limited: number } => ({
  locked: Object.keys(profile.lockedControls).filter((id) => controlIds.has(id)).length,
  limited: Object.keys(profile.limits ?? {}).filter((id) => controlIds.has(id) && !profile.lockedControls[id]).length
})

export const profileSummary = (profile: Pick<LockedControlGroup, 'lockedControls' | 'limits'>, controlIds: Set<string>): string => {
  const { locked, limited } = profileCounts(profile, controlIds)

  return `${locked} locked${limited ? ` · ${limited} limited` : ''}`
}

export function useLockedControls(): {
  lockedControls: typeof lockedControls
  controlLimits: typeof controlLimits
  currentLockedControlsGroup: typeof currentLockedControlsGroup
  defaultLockedControlsGroup: typeof defaultLockedControlsGroup
  setDefaultLockedControlGroup: (groupId: LockedControlGroup['id'] | null) => void
  lockedControlGroups: typeof lockedControlGroups
  getLockedControlGroup: (groupId: LockedControlGroup['id']) => LockedControlGroup | undefined
  // Returns the new profile's id.
  addLockedControlGroup: (lockedControlsGroup: Omit<LockedControlGroup, 'id'>) => string
  updateLockedControlGroup: (
    groupId: LockedControlGroup['id'],
    lockedControlsGroup: Omit<LockedControlGroup, 'id'>
  ) => void
  removeLockedControlGroup: (groupId: LockedControlGroup['id']) => void
} {
  const { avatarDetails } = useAvatarDetails(() => loadLockedControls())
  const avatarId = computed(() => avatarDetails.value?.id)

  const getLockedControlGroup = (groupId: string): LockedControlGroup | undefined => {
    return lockedControlGroups.value.find((g) => g.id === groupId)
  }

  const addLockedControlGroup = (lockedControlsGroup: Omit<LockedControlGroup, 'id'>): string => {
    const id = getUUID()

    lockedControlGroups.value.push({ id, ...lockedControlsGroup })

    saveLockedControlsGroups()

    return id
  }

  const updateLockedControlGroup = (
    groupId: string,
    lockedControlsGroup: Omit<LockedControlGroup, 'id'>
  ): void => {
    const group = lockedControlGroups.value.find((g) => g.id === groupId)

    if (group) {
      group.name = lockedControlsGroup.name
      group.lockedControls = lockedControlsGroup.lockedControls
      group.limits = lockedControlsGroup.limits
    }

    saveLockedControlsGroups()
  }

  const removeLockedControlGroup = (groupId: LockedControlGroup['id']): void => {
    lockedControlGroups.value = lockedControlGroups.value.filter((g) => g.id !== groupId)

    if (currentLockedControlsGroup.value === groupId) currentLockedControlsGroup.value = null
    if (defaultLockedControlsGroup.value === groupId) setDefaultLockedControlGroup(null)

    saveLockedControlsGroups()
  }

  const setDefaultLockedControlGroup = (groupId: LockedControlGroup['id'] | null): void => {
    defaultLockedControlsGroup.value = groupId

    if (groupId) localStorage.setItem(`defaultLockedControlsGroup_${avatarId.value}`, groupId)
    else localStorage.removeItem(`defaultLockedControlsGroup_${avatarId.value}`)
  }

  const saveLockedControlsGroups = (): void => {
    localStorage.setItem(
      `lockedControlsGroups_${avatarId.value}`,
      JSON.stringify(lockedControlGroups.value)
    )
  }

  const loadLockedControls = (): void => {
    const saved = localStorage.getItem(`lockedControlsGroups_${avatarId.value}`)

    if (saved) lockedControlGroups.value = JSON.parse(saved)
    else lockedControlGroups.value = []

    const exists = (id: string | null): id is string => !!id && lockedControlGroups.value.some((g) => g.id === id)

    const storedDefault = localStorage.getItem(`defaultLockedControlsGroup_${avatarId.value}`)

    defaultLockedControlsGroup.value = exists(storedDefault) ? storedDefault : null

    // The last pick for this avatar this session wins, "No profile" included. sessionStorage is
    // empty after a restart, so the first load of a session (or a pick whose profile has since
    // been deleted) falls back to the avatar's default.
    const picked = sessionStorage.getItem(`lockedControlsGroupId_${avatarId.value}`)

    if (picked === NO_PROFILE) currentLockedControlsGroup.value = null
    else if (exists(picked)) currentLockedControlsGroup.value = picked
    else currentLockedControlsGroup.value = defaultLockedControlsGroup.value
  }

  watch(currentLockedControlsGroup, (newGroupId) => {
    sessionStorage.setItem(`lockedControlsGroupId_${avatarId.value}`, newGroupId ?? NO_PROFILE)
  })

  return {
    lockedControls,
    controlLimits,
    currentLockedControlsGroup,
    defaultLockedControlsGroup,
    setDefaultLockedControlGroup,

    lockedControlGroups,
    getLockedControlGroup,
    addLockedControlGroup,
    updateLockedControlGroup,
    removeLockedControlGroup
  }
}
