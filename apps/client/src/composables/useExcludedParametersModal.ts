import ExcludedParametersModal from '@renderer/components/ExcludedParametersModal.vue'

const overlay = useOverlay()
const modal = overlay.create(ExcludedParametersModal)

export function useExcludedParametersModal(): { openModal: () => void } {
  return { openModal: () => modal.open() }
}
