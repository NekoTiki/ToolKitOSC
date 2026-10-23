import { useAddons } from '@renderer/composables/useAddons'
import { setup } from '@renderer/composables/useSetup'
import { api } from '@renderer/lib/tauri-bridge'
import type { CommandWithoutIds, ControlGroup, ControlType, LastUser } from '@toolkitosc/shared-ui'
import { useBhapticsControl, useIntifaceControl, useOpenShockControl, useOwoControl } from '@toolkitosc/shared-ui'
import type { ComputedRef } from 'vue'
import { computed, ref, toRaw, watch } from 'vue'

// The worn avatar's groups as Rust resolved them (see useSetup.ts). Edits change them here so they
// show straight away, then Rust saves them and sends them back resolved.
const controlsList = computed<ControlGroup[]>({
  get: () => setup.value.groups,
  set: (groups) => (setup.value = { ...setup.value, groups })
})

// Rust reports what it did to toys and shockers, whoever asked: the tiles follow, and the existing
// watches in useWebsocketHost.ts pass it on to the share page.
api.onToyValue(({ controlId, value }) => useIntifaceControl().setValue(controlId, value))
api.onShockValue(({ controlId, value }) => useOpenShockControl().setValue(controlId, value))
api.onBhapticsState(({ playing, touched }) => useBhapticsControl().setState({ playing, touched }))
api.onOwoState(({ playing, touched }) => useOwoControl().setState({ playing, touched }))
const controlLastUser = ref<Map<string, Map<string, LastUser>>>(new Map())

export function useControls(onControlsChange?: (controlGroups: ControlGroup[]) => void): {
  controls: ComputedRef<ControlGroup[]>
  // Same groups/controls as `controls`, minus any group marked `hidden` - what the server (and so
  // every share-page viewer) is allowed to know about. Use this, never `controls`, for anything
  // that leaves the host app (see useWebsocketHost.ts).
  visibleControls: ComputedRef<ControlGroup[]>
  controlLastUser: typeof controlLastUser
  getControlLastUser: (groupId: string, controlId: string) => LastUser | undefined
  setControlLastUser: (groupId: string, controlId: string, lastUser: LastUser) => void
  // Returns the new group's id, so the caller can navigate to it.
  addGroup: () => string
  updateGroup: (groupId: string, name: string) => void
  deleteGroup: (groupId: string) => void
  deleteGroups: (groupIds: string[]) => void
  duplicateGroup: (groupId: string) => string | undefined
  setGroups: (groups: ControlGroup[]) => void
  setGroupHidden: (groupId: string, hidden: boolean) => void
  setGroupsHidden: (groupIds: string[], hidden: boolean) => void
  isGroupHidden: (groupId: string) => boolean
  setGroupControls: (groupId: string, controls: ControlType[]) => void
  getControl: (groupId: string, controlId: string) => ControlType | undefined
  addControl: (groupId: string, control: ControlType) => void
  updateControl: (groupId: string, control: ControlType) => void
  deleteControl: (groupId: string, controlId: string) => void
  // Keeps the control's id, so its lock-profile entries, pin and activity history follow it. Goes
  // to the end of the target group unless `index` says where. Returns the index it was moved from.
  moveControl: (fromGroupId: string, controlId: string, toGroupId: string, index?: number) => number | undefined
  setControlPinned: (groupId: string, controlId: string, pinned: boolean) => void

  // Runs a command as you (see handleCommand below).
  handleCommand: (control: ControlType, command: CommandWithoutIds) => void
} {
  const { offAddonForControlType } = useAddons()

  // Each control with the active profile's locks, limits and group rules, whether it's
  // unavailable (add-on off, service down, toys or preset gone) and whether avatar touch has its
  // toy: all worked out in Rust (src-tauri/src/setup/resolve.rs).
  const controls = computed((): ControlGroup[] => controlsList.value)

  // What viewers get. A control whose add-on is off is kept, but viewers don't see it until it's
  // back on. Rust still has it, as unavailable, so a command already on its way is dropped rather
  // than accepted as an unknown control.
  const visibleControls = computed((): ControlGroup[] => {
    return controls.value
      .filter((group) => !group.hidden)
      .map((group) => ({ ...group, controls: group.controls.filter((control) => !offAddonForControlType(control.type)) }))
  })

  const controlsMap = computed<Map<string, Map<string, ControlType>>>(() => {
    const map = new Map<string, Map<string, ControlType>>()

    controls.value.forEach((group) => {
      const groupMap = new Map<string, ControlType>()

      group.controls.forEach((control) => groupMap.set(control.id, control))

      map.set(group.id, groupMap)
    })

    return map
  })

  const getControlLastUser = (groupId: string, controlId: string): LastUser | undefined => {
    return controlLastUser.value.get(groupId)?.get(controlId)
  }

  const setControlLastUser = (groupId: string, controlId: string, lastUser: LastUser): void => {
    if (!controlLastUser.value.has(groupId)) {
      controlLastUser.value.set(groupId, new Map())
    }

    controlLastUser.value.get(groupId)!.set(controlId, lastUser)
  }

  const addGroup = (): string => {
    const id = self.crypto.randomUUID()

    controlsList.value.push({
      id,
      name: `Group ${controls.value.length + 1}`,
      controls: []
    })

    saveControls()

    return id
  }

  const updateGroup = (groupId: string, name: string): void => {
    const group = controlsList.value.find((g) => g.id === groupId)

    if (!group) return

    group.name = name

    saveControls()
  }

  const deleteGroup = (groupId: string): void => {
    controlsList.value = controlsList.value.filter((g) => g.id !== groupId)

    saveControls()
  }

  // A copy right after the original, with new ids for the group and every control in it - control
  // ids key activity and lock profiles, so the copy starts with neither. Pins are dropped too, or
  // Pinned would show every pinned control twice.
  const duplicateGroup = (groupId: string): string | undefined => {
    const index = controlsList.value.findIndex((g) => g.id === groupId)
    const original = controlsList.value[index]

    if (!original) return undefined

    const copy: ControlGroup = {
      ...structuredClone(toRaw(original)),
      id: self.crypto.randomUUID(),
      name: `${original.name} copy`
    }

    copy.controls = copy.controls.map((control) => ({ ...control, id: self.crypto.randomUUID(), pinned: false }))
    controlsList.value.splice(index + 1, 0, copy)

    saveControls()

    return copy.id
  }

  // One state update + one save, not deleteGroup() looped per id - looping would otherwise fire
  // the deep `controls` watch (onControlsChange, saveControls) once per group instead of once for
  // the whole bulk action.
  const deleteGroups = (groupIds: string[]): void => {
    const idsToDelete = new Set(groupIds)

    controlsList.value = controlsList.value.filter((g) => !idsToDelete.has(g.id))

    saveControls()
  }

  // Re-orders the whole group list (drag-and-drop in App.vue), rather than mutating one group at
  // a time like the other setters here - the new order is the only thing that changed, so it's
  // taken as-is instead of matched back up by id.
  const setGroups = (groups: ControlGroup[]): void => {
    // Rust drops the derived flags `controls` carries when it saves.
    controlsList.value = groups

    saveControls()
  }

  const setGroupHidden = (groupId: string, hidden: boolean): void => {
    const group = controlsList.value.find((g) => g.id === groupId)

    if (!group) return

    group.hidden = hidden

    saveControls()
  }

  // Bulk version for the Controls page's multi-select: one state update and one save.
  const setGroupsHidden = (groupIds: string[], hidden: boolean): void => {
    const ids = new Set(groupIds)

    controlsList.value.forEach((group) => {
      if (ids.has(group.id)) group.hidden = hidden
    })

    saveControls()
  }

  const isGroupHidden = (groupId: string): boolean => {
    return controlsList.value.find((g) => g.id === groupId)?.hidden ?? false
  }

  const getControl = (groupId: string, controlId: string): ControlType | undefined => {
    return controlsMap.value.get(groupId)?.get(controlId)
  }

  const setGroupControls = (groupId: string, newControls: ControlType[]): void => {
    const group = controlsList.value.find((g) => g.id === groupId)

    if (!group) return

    group.controls = newControls

    saveControls()
  }

  const addControl = (groupId: string, control: ControlType): void => {
    const group = controlsList.value.find((g) => g.id === groupId)

    if (!group) return

    group.controls.push(control)

    saveControls()
  }

  const updateControl = (groupId: string, control: ControlType): void => {
    const group = controlsList.value.find((g) => g.id === groupId)

    if (!group) return

    const controlIndex = group.controls.findIndex((c) => c.id === control.id)

    if (controlIndex === -1) return

    group.controls[controlIndex] = control

    saveControls()
  }

  const deleteControl = (groupId: string, controlId: string): void => {
    const group = controlsList.value.find((g) => g.id === groupId)

    if (!group) return

    group.controls = group.controls.filter((c) => c.id !== controlId)

    saveControls()
  }

  // One state update + one save, so a viewer never sees the control missing from both groups.
  const moveControl = (fromGroupId: string, controlId: string, toGroupId: string, index?: number): number | undefined => {
    const from = controlsList.value.find((g) => g.id === fromGroupId)
    const to = controlsList.value.find((g) => g.id === toGroupId)
    const fromIndex = from?.controls.findIndex((c) => c.id === controlId) ?? -1

    if (!from || !to || from === to || fromIndex < 0) return undefined

    const [control] = from.controls.splice(fromIndex, 1)

    to.controls.splice(index ?? to.controls.length, 0, control!)

    saveControls()

    return fromIndex
  }

  const setControlPinned = (groupId: string, controlId: string, pinned: boolean): void => {
    const control = controlsList.value.find((g) => g.id === groupId)?.controls.find((c) => c.id === controlId)

    if (!control) return

    control.pinned = pinned

    saveControls()
  }

  // Rust leaves out the derived flags and sends the groups back resolved.
  const saveControls = (): void => {
    const avatarId = setup.value.avatarId

    if (!avatarId) return

    api.setupSaveGroups(avatarId, JSON.parse(JSON.stringify(controlsList.value))).catch((error) => console.error('Could not save the controls:', error))
  }

  // Runs in Rust (src-tauri/src/controls.rs), the same code that runs viewers' commands, without
  // their limits: your own clicks, previews and automations act as you. The tiles follow from the
  // toy-value, shock-value and pattern-value events.
  const handleCommand = (control: ControlType, command: CommandWithoutIds): void => {
    void api.controlRun(control, command)
  }

  watch(
    controls,
    (newControlsGroup) => {
      if (onControlsChange) onControlsChange(newControlsGroup)
    },
    { deep: true }
  )

  return {
    controls,
    visibleControls,

    controlLastUser,
    getControlLastUser,
    setControlLastUser,

    addGroup,
    updateGroup,
    deleteGroup,
    deleteGroups,
    duplicateGroup,
    setGroups,
    setGroupHidden,
    setGroupsHidden,
    isGroupHidden,
    setGroupControls,

    getControl,
    addControl,
    updateControl,
    deleteControl,
    moveControl,
    setControlPinned,

    handleCommand
  }
}
