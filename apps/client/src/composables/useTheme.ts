import type { ColorFamilyWithoutNeutral } from '@vrc-osc-toolkit/shared-ui'
import { applyThemeColor, COLOR_FAMILIES, NEUTRAL_COLOR_FAMILIES } from '@vrc-osc-toolkit/shared-ui'
import { ref } from 'vue'

export { COLOR_FAMILIES, NEUTRAL_COLOR_FAMILIES }
export type { ColorFamily, ColorFamilyWithoutNeutral } from '@vrc-osc-toolkit/shared-ui'

// The desktop client is the theme *authority*: it picks a color, persists it, and broadcasts it
// to remote viewers over the host WS connection (see useWebsocketHost.ts). The DOM-mutating part
// is shared with the server's read-only useClientTheme.ts via applyThemeColor().
const selectedPrimary = ref<ColorFamilyWithoutNeutral | null>(null)
const selectedSecondary = ref<ColorFamilyWithoutNeutral | null>(null)

export function useTheme(): {
  selectedPrimary: typeof selectedPrimary
  selectedSecondary: typeof selectedSecondary
  setTheme: (target: 'primary' | 'secondary', color: ColorFamilyWithoutNeutral) => void
  loadTheme: () => void
} {
  function setTheme(target: 'primary' | 'secondary', color: ColorFamilyWithoutNeutral): void {
    applyThemeColor(target, color)

    if (target === 'primary') selectedPrimary.value = color
    else selectedSecondary.value = color

    localStorage.setItem(`theme_${target}`, color)
  }

  const loadTheme = (): void => {
    const primary = localStorage.getItem('theme_primary')
    const secondary = localStorage.getItem('theme_secondary')

    if (primary) {
      setTheme('primary', primary as ColorFamilyWithoutNeutral)
      selectedPrimary.value = primary as ColorFamilyWithoutNeutral
    }

    if (secondary) {
      setTheme('secondary', secondary as ColorFamilyWithoutNeutral)
      selectedSecondary.value = secondary as ColorFamilyWithoutNeutral
    }
  }

  return { selectedPrimary, selectedSecondary, setTheme, loadTheme }
}
