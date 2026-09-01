// Pure type, no DOM/Vue usage — see types/theme.ts for why this lives outside the composable.
export type OpenShockControlValue = {
  intensity: number
  duration: number
  cooldownEnd: number
  animationDuration: number
}
