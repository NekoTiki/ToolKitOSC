import LockedControlGroupListModal from '@renderer/components/LockedControlGroupListModal.vue'

const overlay = useOverlay()
const modal = overlay.create(LockedControlGroupListModal)

export function useLockedControlGroupListModal(): { openList: () => void } {
  return { openList: () => modal.open() }
}
