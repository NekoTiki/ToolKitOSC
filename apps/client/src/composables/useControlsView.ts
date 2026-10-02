import { useLocalStorage } from '@vueuse/core'
import { ref } from 'vue'

export type TileDensity = 's' | 'm' | 'l'

// The user's tile size (S/M/L), persisted - it depends on how big their overlay window is, which
// rarely changes. Also offered as the default in Settings → Appearance.
const density = useLocalStorage<TileDensity>('controls_tileDensity', 'm')

// Use vs Edit mode on the Controls page. Module-level so it survives leaving for the control
// editor and coming back; deliberately not persisted, so the app always opens in Use mode where
// nothing can be moved or deleted by accident.
const editMode = ref(false)

export function useControlsView(): { density: typeof density; editMode: typeof editMode } {
  return { density, editMode }
}
