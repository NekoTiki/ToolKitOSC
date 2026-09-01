import { useControls } from '@renderer/composables/useControls'
import type { ControlType } from '@vrc-osc-toolkit/shared-ui'
import _ from 'lodash'
import { ref } from 'vue'

export const getUUID = (): string => {
  return self.crypto.randomUUID()
}

const defaultControl: () => Partial<ControlType> = () => ({
  type: 'boolean',
  name: ''
})

const model = ref<Partial<ControlType>>({})
const open = ref(false)
const _groupId = ref<string>()

export function useControlModal(): {
  model: typeof model
  open: typeof open
  openModal: (groupId: string, controlId?: string | null) => void
  submit: () => void
} {
  const { getControl, addControl, updateControl } = useControls()

  const resetForm = (): void => {
    model.value = defaultControl()
    _groupId.value = undefined
  }

  const openModal = (groupId: string, controlId: string | null = null): void => {
    resetForm()

    _groupId.value = groupId

    if (controlId) model.value = _.cloneDeep(getControl(groupId, controlId)) || defaultControl()

    open.value = true
  }

  const submit = (): void => {
    if (!_groupId.value) return

    const modelData = model.value

    if (!modelData.id) {
      modelData.id = getUUID()
      addControl(_groupId.value, model.value as ControlType)
    } else updateControl(_groupId.value, model.value as ControlType)

    open.value = false

    resetForm()
  }

  return { model, open, openModal, submit }
}
