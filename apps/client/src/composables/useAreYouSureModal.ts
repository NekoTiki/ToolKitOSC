import AreYouSureModal from '@renderer/components/AreYouSureModal.vue'

export type AreYouSureOptions = {
  title: string
  message: string
  confirmText: string
  cancelText: string
}

const defaultOptions = (): AreYouSureOptions => ({
  title: 'Are you sure?',
  message: 'This action cannot be undone.',
  confirmText: 'Confirm',
  cancelText: 'Cancel'
})

// create() once at module scope - reused across every openModal() call rather than registering a
// new overlay each time, the same singleton-instance shape every other use<Feature>Modal.ts in
// this app follows.
const overlay = useOverlay()
const modal = overlay.create(AreYouSureModal)

export function useAreYouSureModal(): {
  openModal: (opts?: Partial<AreYouSureOptions>) => Promise<boolean>
} {
  const openModal = (opts?: Partial<AreYouSureOptions>): Promise<boolean> => {
    return modal.open({ ...defaultOptions(), ...opts }).result
  }

  return { openModal }
}
