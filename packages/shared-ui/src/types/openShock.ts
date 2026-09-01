// Pure type, no DOM/Vue usage — see types/theme.ts for why this lives outside the composable.
export type OpenShockControlValue = {
  intensity: number
  duration: number
  cooldownEnd: number
  animationDuration: number
}

// What actually got sent to OpenShock for one 'open-shock-shocker' command - the intensity/duration
// are randomized (within the control's configured range) at command-handling time, not known ahead
// of time, so this is handed back from `handleCommand` for the caller to log alongside the command
// itself. `shockers` are display names, resolved from `OpenShockControl['shockers']` (ids) at the
// moment the command fires - not the ids themselves, and not re-resolved later when a log is read.
export type OpenShockCommandResult = {
  intensity: number
  duration: number
  shockers: string[]
}
