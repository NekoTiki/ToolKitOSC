import { controlGroups } from '@renderer/assets/controlsExample'
import { useAvatarDetails } from '@renderer/composables/useAvatarDetails'
import { useLockedControls } from '@renderer/composables/useLockedControls'
import { useOpenShock } from '@renderer/composables/useOpenShock'
import { useOscMessages } from '@renderer/composables/useOscMessages'
import type {
  BooleanEnumControl,
  CommandWithoutIds,
  ControlGroup,
  ControlType,
  EnumControl,
  LastUser
} from '@vrc-osc-toolkit/shared-ui'
import { useOpenShockControl } from '@vrc-osc-toolkit/shared-ui'
import type { ComputedRef } from 'vue'
import { computed, ref, watch } from 'vue'

const controlsList = ref<ControlGroup[]>([])
const controlLastUser = ref<Map<string, Map<string, LastUser>>>(new Map())

export function useControls(onControlsChange?: (controlGroups: ControlGroup[]) => void): {
  controls: ComputedRef<ControlGroup[]>
  controlLastUser: typeof controlLastUser
  getControlLastUser: (groupId: string, controlId: string) => LastUser | undefined
  setControlLastUser: (groupId: string, controlId: string, lastUser: LastUser) => void
  addGroup: () => void
  updateGroup: (groupId: string, name: string) => void
  deleteGroup: (groupId: string) => void
  setGroupControls: (groupId: string, controls: ControlType[]) => void
  getControl: (groupId: string, controlId: string) => ControlType | undefined
  addControl: (groupId: string, control: ControlType) => void
  updateControl: (groupId: string, control: ControlType) => void
  deleteControl: (groupId: string, controlId: string) => void
  loadControls: (avatarId: string) => void

  handleCommand: (control: ControlType, command: CommandWithoutIds) => void
} {
  const { avatarDetails } = useAvatarDetails(() => loadControls())
  const { lockedControls } = useLockedControls()
  const { command: osCommand } = useOpenShock()
  const { controlValue, setValue } = useOpenShockControl()
  const { update } = useOscMessages()

  const controls = computed((): ControlGroup[] => {
    return controlsList.value.map((controlGroup) => ({
      ...controlGroup,
      controls: controlGroup.controls.map((control) => ({
        ...control,
        locked: lockedControls.value[control.id] ?? false
      }))
    }))
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

  const addGroup = (): void => {
    controlsList.value.push({
      id: self.crypto.randomUUID(),
      name: `Group ${controls.value.length + 1}`,
      controls: []
    })

    saveControls()
  }

  const updateGroup = (groupId: string, name: string): void => {
    const group = controlsList.value.find((g) => g.id === groupId)

    if (!group) return

    group.name = name

    saveControls()
  }

  const deleteGroup = (groupId: string): void => {
    controlsList.value = controls.value.filter((g) => g.id !== groupId)

    saveControls()
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

  const loadControls = (): void => {
    if (avatarId.value === 'avtr_b89f691a-d31d-4a84-83fe-5cff212b3761') {
      controlsList.value = controlGroups
    } else {
      const storedControls = localStorage.getItem(`controls_${avatarId.value}`)

      controlsList.value = storedControls ? JSON.parse(storedControls) : []
    }
  }

  const saveControls = (): void => {
    localStorage.setItem(`controls_${avatarId.value}`, JSON.stringify(controls.value))
  }

  const handleCommand = (control: ControlType, command: CommandWithoutIds): void => {
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
    } else if (command.type === 'slider' && control?.type === 'slider') {
      update({ address: control.inputAddress, args: [command.value] })
    } else if (command.type === 'open-shock-shocker' && control?.type === 'open-shock-shocker') {
      const piShockValue = controlValue.value.get(control.id)
      if (piShockValue && piShockValue.cooldownEnd > Date.now()) return

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
    }
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

    controlLastUser,
    getControlLastUser,
    setControlLastUser,

    addGroup,
    updateGroup,
    deleteGroup,
    setGroupControls,

    getControl,
    addControl,
    updateControl,
    deleteControl,

    loadControls,
    handleCommand
  }
}
