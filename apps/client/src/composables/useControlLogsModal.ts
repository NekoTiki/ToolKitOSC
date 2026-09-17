import ControlLogsModal from '@renderer/components/ControlLogsModal.vue'

const overlay = useOverlay()
const modal = overlay.create(ControlLogsModal)

export function useControlLogsModal(): {
  openModal: (controlId: string, controlName: string) => void
} {
  return { openModal: (controlId, controlName) => modal.open({ controlId, controlName }) }
}
