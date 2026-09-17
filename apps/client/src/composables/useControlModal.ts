import ControlModal from '@renderer/components/ControlModal.vue'

export const getUUID = (): string => {
  return self.crypto.randomUUID()
}

const overlay = useOverlay()
const modal = overlay.create(ControlModal)

export function useControlModal(): {
  openModal: (groupId: string, controlId?: string | null) => void
} {
  const openModal = (groupId: string, controlId: string | null = null): void => {
    modal.open({ groupId, controlId })
  }

  return { openModal }
}
