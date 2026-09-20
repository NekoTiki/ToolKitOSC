import ManageGroupsModal from '@renderer/components/ManageGroupsModal.vue'

// create() once at module scope - same singleton-instance shape every other use<Feature>Modal.ts
// in this app follows (see useAiControlsModal.ts, useAreYouSureModal.ts).
const overlay = useOverlay()
const modal = overlay.create(ManageGroupsModal)

export function useManageGroupsModal(): { openModal: () => void } {
  return { openModal: () => modal.open({}) }
}
