import { ref } from 'vue'

const open = ref(false)
const callback = ref<(result: boolean) => void>()

type Options = {
  title: string
  message: string
  confirmText: string
  cancelText: string
}

const defaultOptions = (): Options => ({
  title: 'Are you sure?',
  message: 'This action cannot be undone.',
  confirmText: 'Confirm',
  cancelText: 'Cancel'
})

const options = ref<Options>(defaultOptions())

export function useAreYouSureModal(): {
  open: typeof open
  options: typeof options
  openModal: (opts?: Partial<Options>) => Promise<boolean>
  confirm: () => void
  cancel: () => void
} {
  function openModal(opts?: Partial<Options>): Promise<boolean> {
    open.value = true
    options.value = { ...defaultOptions(), ...opts }

    return new Promise((resolve) => {
      callback.value = resolve
    })
  }

  function confirm(): void {
    open.value = false
    callback.value?.(true)
  }

  function cancel(): void {
    open.value = false
    callback.value?.(false)
  }

  return { open, options, openModal, confirm, cancel }
}
