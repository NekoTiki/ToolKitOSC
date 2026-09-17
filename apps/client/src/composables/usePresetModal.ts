import { usePresets } from '@renderer/composables/usePresets'
import type { PresetParameter } from '@vrc-osc-toolkit/shared-ui'
import _ from 'lodash'
import { ref } from 'vue'

export type PresetModalModel = {
  id?: string
  name: string
  icon?: string
  parameters: PresetParameter[]
}

const defaultModel = (): PresetModalModel => ({ name: '', icon: '', parameters: [] })

const model = ref<PresetModalModel>(defaultModel())
const open = ref(false)
const listOpen = ref(false)

export function usePresetModal(): {
  model: typeof model
  open: typeof open
  listOpen: typeof listOpen
  openModal: (presetId?: string) => void
  openCreateFromCurrentState: () => void
  captureIntoModel: () => void
  submit: () => void
} {
  const { getPreset, addPreset, updatePreset, captureParameters } = usePresets()

  const resetForm = (): void => {
    model.value = defaultModel()
  }

  const openModal = (presetId?: string): void => {
    resetForm()

    if (presetId) {
      const preset = getPreset(presetId)

      if (preset) {
        model.value = {
          id: preset.id,
          name: preset.name,
          icon: preset.icon,
          parameters: _.cloneDeep(preset.parameters)
        }
      }
    }

    open.value = true
  }

  const openCreateFromCurrentState = (): void => {
    resetForm()

    model.value.parameters = captureParameters()

    open.value = true
  }

  // Re-syncs every currently-captured, non-excluded parameter into the model - adding newly-seen
  // ones and refreshing values for ones already present, without dropping a parameter the user
  // added by hand (see PresetModal.vue's "Add Parameter" picker) that VRChat hasn't reported a
  // live value for yet.
  const captureIntoModel = (): void => {
    const captured = captureParameters()
    const byAddress = new Map(model.value.parameters.map((param) => [param.address, param]))

    captured.forEach((param) => byAddress.set(param.address, param))

    model.value.parameters = Array.from(byAddress.values())
  }

  const submit = (): void => {
    if (!model.value.name.trim()) return

    if (!model.value.id) {
      addPreset(model.value.name, model.value.icon, model.value.parameters)
    } else {
      updatePreset(model.value.id, {
        name: model.value.name,
        icon: model.value.icon,
        parameters: model.value.parameters
      })
    }

    open.value = false

    resetForm()
  }

  return { model, open, listOpen, openModal, openCreateFromCurrentState, captureIntoModel, submit }
}
