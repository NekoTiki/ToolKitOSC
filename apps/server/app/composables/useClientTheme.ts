import type { ColorFamily } from '@vrc-osc-toolkit/shared-ui'
import { applyThemeColor, clearThemeColor } from '@vrc-osc-toolkit/shared-ui'

// Viewer-side wrapper around the shared theme core: this app never picks a theme itself, it
// only applies/clears whatever the host broadcasts over `theme-update`.
export function useClientTheme(): {
  setTheme: (target: 'primary' | 'secondary', color: ColorFamily) => void
  removeTheme: () => void
} {
  function setTheme(target: 'primary' | 'secondary', color: ColorFamily): void {
    applyThemeColor(target, color)
  }

  function removeTheme(): void {
    clearThemeColor('primary')
    clearThemeColor('secondary')
  }

  return { setTheme, removeTheme }
}
