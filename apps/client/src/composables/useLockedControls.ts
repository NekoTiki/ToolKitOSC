import { setup } from '@renderer/composables/useSetup'
import type { LockedControlGroup } from '@renderer/lib/tauri-bridge'
import { api } from '@renderer/lib/tauri-bridge'
import { getUUID } from '@renderer/utils/uuid'
import { computed } from 'vue'

export type { GroupOverrides, LockedControlGroup } from '@renderer/lib/tauri-bridge'

// The worn avatar's profiles, its default and the one that's on, as Rust keeps them (see
// useSetup.ts). The last pick for an avatar since the app started wins, "No profile" included;
// until then it's the avatar's default.
const lockedControlGroups = computed<LockedControlGroup[]>({
  get: () => setup.value.profiles,
  set: (profiles) => (setup.value = { ...setup.value, profiles })
})
const defaultLockedControlsGroup = computed(() => setup.value.defaultProfile)
const currentLockedControlsGroup = computed<LockedControlGroup['id'] | null>({
  get: () => setup.value.activeProfile,
  set: (id) => {
    setup.value = { ...setup.value, activeProfile: id }
    api.setupSwitchProfile(id).catch((error) => console.error('Could not switch the profile:', error))
  }
})
const activeProfile = computed(() => lockedControlGroups.value.find((g) => g.id === currentLockedControlsGroup.value))
const lockedControls = computed<LockedControlGroup['lockedControls']>(() => activeProfile.value?.lockedControls ?? {})
const controlLimits = computed<NonNullable<LockedControlGroup['limits']>>(() => activeProfile.value?.limits ?? {})
const groupOverrides = computed<NonNullable<LockedControlGroup['groupOverrides']>>(() => activeProfile.value?.groupOverrides ?? {})

// How many of the controls that exist now (`controlIds`) have a rule for each group, by group id.
// Groups with none are left out.
export const groupOverrideCounts = (profile: Pick<LockedControlGroup, 'groupOverrides'>, controlIds: Set<string>): Record<string, number> =>
  Object.fromEntries(
    Object.entries(profile.groupOverrides ?? {})
      .map(([groupId, rules]) => [groupId, Object.keys(rules).filter((id) => controlIds.has(id)).length] as const)
      .filter(([, count]) => count > 0)
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

// `groups` names the viewer groups, so the summary can say "Friends +2" for a group with rules on
// two controls. Rules for deleted groups aren't mentioned.
export const profileSummary = (
  profile: Pick<LockedControlGroup, 'lockedControls' | 'limits' | 'groupOverrides'>,
  controlIds: Set<string>,
  groups: { id: string; name: string }[] = []
): string => {
  const { locked, limited } = profileCounts(profile, controlIds)
  const counts = groupOverrideCounts(profile, controlIds)
  const forGroups = groups.filter((group) => counts[group.id]).map((group) => `${group.name} +${counts[group.id]}`)

  return [`${locked} locked`, limited ? `${limited} limited` : '', ...forGroups].filter(Boolean).join(' · ')
}

export function useLockedControls(): {
  lockedControls: typeof lockedControls
  controlLimits: typeof controlLimits
  groupOverrides: typeof groupOverrides
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
  const getLockedControlGroup = (groupId: string): LockedControlGroup | undefined => {
    return lockedControlGroups.value.find((g) => g.id === groupId)
  }

  // Rust saves them and sends them back. Deleting the profile that's on leaves none on.
  const save = (profiles: LockedControlGroup[], defaultProfile: string | null): void => {
    const avatarId = setup.value.avatarId

    if (!avatarId) return

    setup.value = { ...setup.value, profiles, defaultProfile }
    api
      .setupSaveProfiles(avatarId, JSON.parse(JSON.stringify(profiles)), defaultProfile)
      .catch((error) => console.error('Could not save the profiles:', error))
  }

  const addLockedControlGroup = (lockedControlsGroup: Omit<LockedControlGroup, 'id'>): string => {
    const id = getUUID()

    save([...lockedControlGroups.value, { id, ...lockedControlsGroup }], defaultLockedControlsGroup.value)

    return id
  }

  const updateLockedControlGroup = (groupId: string, lockedControlsGroup: Omit<LockedControlGroup, 'id'>): void => {
    save(
      lockedControlGroups.value.map((g) => (g.id === groupId ? { id: groupId, ...lockedControlsGroup } : g)),
      defaultLockedControlsGroup.value
    )
  }

  const removeLockedControlGroup = (groupId: LockedControlGroup['id']): void => {
    if (currentLockedControlsGroup.value === groupId) setup.value = { ...setup.value, activeProfile: null }

    save(
      lockedControlGroups.value.filter((g) => g.id !== groupId),
      defaultLockedControlsGroup.value === groupId ? null : defaultLockedControlsGroup.value
    )
  }

  const setDefaultLockedControlGroup = (groupId: LockedControlGroup['id'] | null): void => {
    save(lockedControlGroups.value, groupId)
  }

  return {
    lockedControls,
    controlLimits,
    groupOverrides,
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
