import { ref } from 'vue'

const open = ref(false)

export function useSettingsModal(): {
  open: typeof open
  openModal: () => void
} {
  const openModal = (): void => {
    open.value = true
  }

  return { open, openModal }
}
