import { controlGroups } from '@renderer/assets/controlsExample'
import { useAvatarDetails } from '@renderer/composables/useAvatarDetails'
import { useIntiface } from '@renderer/composables/useIntiface'
import { useIntifacePatterns } from '@renderer/composables/useIntifacePatterns'
import { useLockedControls } from '@renderer/composables/useLockedControls'
import { useOpenShock } from '@renderer/composables/useOpenShock'
import { useOscMessages } from '@renderer/composables/useOscMessages'
import { usePresets } from '@renderer/composables/usePresets'
import type {
  BooleanEnumControl,
  CommandWithoutIds,
  ControlGroup,
  ControlType,
  EnumControl,
  LastUser,
  OpenShockCommandResult,
  StepEnumControl
} from '@toolkitosc/shared-ui'
import { useIntifaceControl, useOpenShockControl } from '@toolkitosc/shared-ui'
import _ from 'lodash'
import type { ComputedRef } from 'vue'
import { computed, ref, toRaw, watch } from 'vue'

const controlsList = ref<ControlGroup[]>([])
const controlLastUser = ref<Map<string, Map<string, LastUser>>>(new Map())

// One-off migration: 'intiface-vibrator' (with a `vibrators` field) was renamed to 'intiface-toy'
// (with `actuators`) - a control saved locally under the old shape doesn't just look different,
// Control.vue's dispatcher only recognizes the current type string, so it silently renders nothing
// at all. Straightens any such control back into the current shape instead of leaving it invisible.
const migrateControlGroups = (groups: ControlGroup[]): ControlGroup[] => {
  return groups.map((group) => ({
    ...group,
    controls: group.controls.map((control) => {
      const legacy = control as unknown as { type: string; vibrators?: unknown }

      if (legacy.type !== 'intiface-vibrator') return control

      const { vibrators, ...rest } = legacy

      return { ...rest, type: 'intiface-toy', actuators: vibrators } as ControlType
    })
  }))
}

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
  loadControls: (avatarId: string) => void

  // Returns the resolved OpenShock intensity/duration/shockers for an 'open-shock-shocker' command
  // (see OpenShockCommandResult) so the caller can log what actually got sent - it isn't known ahead
  // of time, since it's randomized here. undefined for every other command type, or when the
  // command didn't actually fire (OpenShock unavailable, or still on cooldown).
  handleCommand: (
    control: ControlType,
    command: CommandWithoutIds
  ) => OpenShockCommandResult | undefined
} {
  const { avatarDetails } = useAvatarDetails(() => loadControls())
  const { lockedControls, controlLimits } = useLockedControls()
  const { command: osCommand, isAvailable: openShockAvailable, shockerNames } = useOpenShock()
  const { controlValue, setValue } = useOpenShockControl()
  const {
    setActuatorIntensity,
    isAvailable: intifaceAvailable,
    devices: intifaceDevices
  } = useIntiface()
  const { setValue: setIntifaceValue } = useIntifaceControl()
  const { playPattern, releaseActuators } = useIntifacePatterns()
  const { update } = useOscMessages()
  const { getPreset } = usePresets()

  const controls = computed((): ControlGroup[] => {
    return controlsList.value.map((controlGroup) => ({
      ...controlGroup,
      controls: controlGroup.controls.map((control) => ({
        ...control,
        locked: lockedControls.value[control.id] ?? false,
        limits: controlLimits.value[control.id],
        // OpenShock/Intiface controls are unavailable to everyone - same as a locked control -
        // unless the service is actually reachable (see useOpenShock.ts/useIntiface.ts). An
        // Intiface toy/pattern control is additionally unavailable when every toy it targets has
        // been unplugged/disconnected - even while Intiface itself is still connected - since
        // there's nothing left for it to actuate; a control spanning several toys stays available
        // as long as at least one of them is still present. Other control types never depend on an
        // external service, so they're always available.
        unavailable:
          control.type === 'open-shock-shocker'
            ? !openShockAvailable.value
            : control.type === 'intiface-toy' || control.type === 'intiface-pattern'
              ? !intifaceAvailable.value ||
                control.actuators.every(
                  (actuator) => !intifaceDevices.value.has(actuator.deviceIndex)
                )
              : control.type === 'preset'
                ? !getPreset(control.presetId)
                : false
      }))
    }))
  })

  const visibleControls = computed((): ControlGroup[] => {
    return controls.value.filter((group) => !group.hidden)
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

  const avatarId = computed(() => avatarDetails.value?.id)

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
    // Callers pass groups from `controls`, so drop the derived flags it adds.
    controlsList.value = groups.map((group) => ({
      ...group,
      controls: group.controls.map((control) => _.omit(control, ['locked', 'unavailable', 'limits']) as ControlType)
    }))

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
    console.log('Changed')

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

  // Writes the raw stored control, not a copy from `controls` - that computed adds derived
  // locked/unavailable flags that must never be saved.
  const setControlPinned = (groupId: string, controlId: string, pinned: boolean): void => {
    const control = controlsList.value.find((g) => g.id === groupId)?.controls.find((c) => c.id === controlId)

    if (!control) return

    control.pinned = pinned

    saveControls()
  }

  const loadControls = (): void => {
    if (avatarId.value === 'avtr_b89f691a-d31d-4a84-83fe-5cff212b3761') {
      controlsList.value = controlGroups
    } else {
      const storedControls = localStorage.getItem(`controls_${avatarId.value}`)

      controlsList.value = storedControls ? migrateControlGroups(JSON.parse(storedControls)) : []
    }
  }

  // The raw list, not `controls`: the derived locked/unavailable/limits flags must never be saved.
  const saveControls = (): void => {
    localStorage.setItem(`controls_${avatarId.value}`, JSON.stringify(controlsList.value))
  }

  const handleCommand = (
    control: ControlType,
    command: CommandWithoutIds
  ): OpenShockCommandResult | undefined => {
    if (command.type === 'boolean' && control?.type === 'boolean') {
      update({ address: control.inputAddress, args: [command.value] })
    } else if (command.type === 'boolean-group' && control?.type === 'boolean-group') {
      control.inputs.forEach((input) => {
        update({ address: input.inputAddress, args: [command.value] })
      })
    } else if (command.type === 'boolean-enum' && control?.type === 'boolean-enum') {
      const selectedInput: BooleanEnumControl['inputs'][number] =
        control.inputs[command.value as number]

      if (selectedInput) {
        selectedInput.inputAddress.true.forEach((addr) => update({ address: addr, args: [true] }))
        selectedInput.inputAddress.false.forEach((addr) => update({ address: addr, args: [false] }))
      }
    } else if (command.type === 'enum' && control?.type === 'enum') {
      const selectedOption: EnumControl['options'][number] =
        control.options[command.value as number]

      if (selectedOption) update({ address: control.inputAddress, args: [selectedOption.value] })
    } else if (command.type === 'step-enum' && control?.type === 'step-enum') {
      const selectedOption: StepEnumControl['options'][number] =
        control.options[command.value as number]

      if (selectedOption) update({ address: control.inputAddress, args: [selectedOption.value] })
    } else if (command.type === 'slider' && control?.type === 'slider') {
      update({ address: control.inputAddress, args: [command.value] })
    } else if (command.type === 'open-shock-shocker' && control?.type === 'open-shock-shocker') {
      // Belt-and-suspenders: the UI already hides/blocks this control while OpenShock isn't
      // configured (see `unavailable` above and useWebsocketHost.ts's enforcement), but guard here
      // too in case handleCommand is ever called directly (e.g. from the authoring preview).
      if (!openShockAvailable.value) return undefined

      const piShockValue = controlValue.value.get(control.id)
      if (piShockValue && piShockValue.cooldownEnd > Date.now()) return undefined

      const intensity = Math.floor(
        Math.random() * (control.intensity.max - control.intensity.min) + control.intensity.min
      )
      const duration = Math.floor(
        Math.random() * (control.duration.max - control.duration.min) + control.duration.min
      )

      setValue(control.id, {
        intensity: Number(intensity),
        duration: Number(duration),
        cooldownEnd: Date.now() + control.cooldown + control.animationDuration,
        animationDuration: control.animationDuration
      })

      setTimeout(() => {
        const cmd = control.shockers.map((shocker) => ({
          id: shocker,
          type: control.mode,
          duration: Number(duration),
          intensity: Number(intensity)
        }))

        void osCommand(cmd)
      }, control.animationDuration)

      return {
        intensity: Number(intensity),
        duration: Number(duration),
        // Resolved to display names right now, from the cache useOpenShock keeps warm - not the
        // raw ids (control.shockers), and not left for the log viewer to resolve later: a shocker
        // could be renamed or removed from the account by the time a log gets read. A name that
        // isn't cached yet is left out entirely rather than falling back to the id.
        shockers: control.shockers
          .map((id) => shockerNames.value.get(id))
          .filter((name): name is string => !!name)
      }
    } else if (command.type === 'intiface-toy' && control?.type === 'intiface-toy') {
      // Same belt-and-suspenders guard as OpenShock above.
      if (!intifaceAvailable.value) return undefined

      setIntifaceValue(control.id, command.value)
      setActuatorIntensity(control.actuators, command.value)
      // Driving these actuators directly wins over anything currently animating them - stop any
      // pattern control that was still running against one of them (see the task's requirement
      // that manually controlling a toy from elsewhere turns its pattern off).
      releaseActuators(control.actuators)
    } else if (command.type === 'intiface-pattern' && control?.type === 'intiface-pattern') {
      // Same belt-and-suspenders guard as OpenShock above.
      if (!intifaceAvailable.value) return undefined

      playPattern(control.id, command.value, control.actuators)
    } else if (command.type === 'preset' && control?.type === 'preset') {
      const preset = getPreset(control.presetId)

      preset?.parameters.forEach((param) => update({ address: param.address, args: [param.value] }))
    }

    return undefined
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

    loadControls,
    handleCommand
  }
}
