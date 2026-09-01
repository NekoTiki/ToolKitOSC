import { ref } from 'vue'

const open = ref(false)

export function useClientsListDrawer(): {
  open: typeof open
  openDrawer: () => void
} {
  const openDrawer = (): void => {
    open.value = true
  }

  return { open, openDrawer }
}
