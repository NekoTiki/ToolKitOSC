import AiControlsModal from '@renderer/components/AiControlsModal.vue'

const overlay = useOverlay()
const modal = overlay.create(AiControlsModal)

export function useAiControlsModal(): { openModal: () => void } {
  return { openModal: () => modal.open({}) }
}
