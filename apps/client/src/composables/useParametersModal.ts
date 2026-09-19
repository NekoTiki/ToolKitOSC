import ParametersModal from '@renderer/components/ParametersModal.vue'

const overlay = useOverlay()
const modal = overlay.create(ParametersModal)

export function useParametersModal(): { openModal: () => void } {
  return { openModal: () => modal.open() }
}
