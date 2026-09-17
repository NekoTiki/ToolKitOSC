import LockedControlGroupModal from '@renderer/components/LockedControlGroupModal.vue'

export const getUUID = (): string => {
  return self.crypto.randomUUID()
}

const overlay = useOverlay()
const modal = overlay.create(LockedControlGroupModal)

export function useLockedControlGroupModal(): {
  openModal: (groupId?: string) => void
} {
  return { openModal: (groupId) => modal.open({ groupId }) }
}
