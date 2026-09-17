import PresetModal from '@renderer/components/PresetModal.vue'

const overlay = useOverlay()
const modal = overlay.create(PresetModal)

export function usePresetModal(): { openModal: (presetId?: string) => void } {
  return { openModal: (presetId) => modal.open({ presetId }) }
}
