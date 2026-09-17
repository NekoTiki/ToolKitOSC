import PresetListModal from '@renderer/components/PresetListModal.vue'

const overlay = useOverlay()
const modal = overlay.create(PresetListModal)

export function usePresetListModal(): { openList: () => void } {
  return { openList: () => modal.open() }
}
