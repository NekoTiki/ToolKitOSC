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

const currentLockedControlsGroup = ref<LockedControlGroup['id'] | null>(null)
const lockedControlGroups = ref<LockedControlGroup[]>([])
const lockedControls = computed<LockedControlGroup['lockedControls']>(
  () =>
    lockedControlGroups.value.find((g) => g.id === currentLockedControlsGroup.value)
      ?.lockedControls ?? {}
)
const controlLimits = computed<NonNullable<LockedControlGroup['limits']>>(
  () => lockedControlGroups.value.find((g) => g.id === currentLockedControlsGroup.value)?.limits ?? {}
)

export function useLockedControls(): {
  lockedControls: typeof lockedControls
  controlLimits: typeof controlLimits
  currentLockedControlsGroup: typeof currentLockedControlsGroup
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

    saveLockedControlsGroups()
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

    currentLockedControlsGroup.value = sessionStorage.getItem(
      `lockedControlsGroupId_${avatarId.value}`
    )
  }

  watch(currentLockedControlsGroup, (newGroupId) => {
    if (newGroupId) sessionStorage.setItem(`lockedControlsGroupId_${avatarId.value}`, newGroupId)
    else sessionStorage.removeItem(`lockedControlsGroupId_${avatarId.value}`)
  })

  return {
    lockedControls,
    controlLimits,
    currentLockedControlsGroup,

    lockedControlGroups,
    getLockedControlGroup,
    addLockedControlGroup,
    updateLockedControlGroup,
    removeLockedControlGroup
  }
}
