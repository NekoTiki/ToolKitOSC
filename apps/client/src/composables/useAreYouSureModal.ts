import AreYouSureModal from '@renderer/components/AreYouSureModal.vue'

export type AreYouSureOptions = {
  title: string
  message: string
  confirmText: string
  cancelText: string
  // Destructive (the default): warning icon and a red confirm button. false for harmless ones
  // like unbanning, which get the primary color instead.
  danger: boolean
}

const defaultOptions = (): AreYouSureOptions => ({
  title: 'Are you sure?',
  message: 'This action cannot be undone.',
  confirmText: 'Confirm',
  cancelText: 'Cancel',
  danger: true
})

// Created once and reused across every openModal() call rather than registering a new overlay each
// time. Lazily, from the first useAreYouSureModal() call: useOverlay() injects, so it has to run
// inside a component's setup(), not at module import.
let modal: ReturnType<ReturnType<typeof useOverlay>['create']> | undefined

export function useAreYouSureModal(): {
  openModal: (opts?: Partial<AreYouSureOptions>) => Promise<boolean>
} {
  modal ??= useOverlay().create(AreYouSureModal)

  const openModal = (opts?: Partial<AreYouSureOptions>): Promise<boolean> => {
    return modal!.open({ ...defaultOptions(), ...opts }).result
  }

  return { openModal }
}
