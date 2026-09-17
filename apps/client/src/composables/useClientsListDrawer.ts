import ClientsListSliderover from '@renderer/components/ClientsListSliderover.vue'

const overlay = useOverlay()
const modal = overlay.create(ClientsListSliderover)

export function useClientsListDrawer(): { openDrawer: () => void } {
  return { openDrawer: () => modal.open() }
}
