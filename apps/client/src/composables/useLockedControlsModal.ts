import type { LockedControlGroup } from '@renderer/composables/useLockedControls'
import { useLockedControls } from '@renderer/composables/useLockedControls'
import _ from 'lodash'
import { ref } from 'vue'

export const getUUID = (): string => {
  return self.crypto.randomUUID()
}

type LockedControlGroupData = Omit<LockedControlGroup, 'id'> &
  Partial<Pick<LockedControlGroup, 'id'>>

const defaultControl: () => LockedControlGroupData = () => ({
  name: '',
  lockedControls: {}
})

const model = ref<LockedControlGroupData>(defaultControl())
const open = ref(false)
const listOpen = ref(false)

export function useLockedControlsModal(): {
  model: typeof model
  open: typeof open
  listOpen: typeof listOpen
  openModal: (groupId?: string) => void
  submit: () => void
} {
  const { getLockedControlGroup, addLockedControlGroup, updateLockedControlGroup } =
    useLockedControls()

  const resetForm = (): void => {
    model.value = defaultControl()
  }

  const openModal = (groupId?: string): void => {
    resetForm()

    if (groupId) model.value = _.cloneDeep(getLockedControlGroup(groupId)) || defaultControl()

    open.value = true
  }

  const submit = (): void => {
    const modelData = model.value

    if (!modelData.id) {
      modelData.id = getUUID()
      addLockedControlGroup(model.value)
    } else updateLockedControlGroup(modelData.id, model.value)

    open.value = false

    resetForm()
  }

  return { model, open, listOpen, openModal, submit }
}
