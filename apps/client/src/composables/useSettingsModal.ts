import SettingsModal from '@renderer/components/SettingsModal.vue'

const overlay = useOverlay()
const modal = overlay.create(SettingsModal)

export function useSettingsModal(): { openModal: () => void } {
  return { openModal: () => modal.open() }
}
