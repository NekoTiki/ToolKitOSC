import type { ControlType, ControlTypes } from '../types/controls'
import { INTIFACE_PATTERN_OFF_ID } from '../types/controls'
import type { OpenShockCommandResult } from '../types/openShock'
import type { CommandValue } from '../types/protocol'

// Shared by every place that renders a logged command (the desktop app's Activity and Viewers
// pages and its Controls page log line, and the share page's last-action bar).
export const isOpenShockCommandValue = (value: CommandValue | undefined): value is OpenShockCommandResult =>
  typeof value === 'object' && value !== null && 'shockers' in value

export const formatCommandValue = (value: CommandValue | undefined): string => {
  if (isOpenShockCommandValue(value)) {
    return `${value.intensity}% • ${(value.duration / 1000).toFixed(1)}s`
  }
  if (typeof value === 'boolean') return value ? 'On' : 'Off'
  if (typeof value === 'number') return Number.isInteger(value) ? String(value) : value.toFixed(2)
  if (value === undefined) return '-'

  return String(value)
}

// What a command did, in words, for someone watching: "turned on", "set to Blush", "set to 64%".
// Option labels come from the control when it's known; without it, falls back to the raw value.
export const describeCommand = (control: ControlType | undefined, type: ControlTypes, value: CommandValue | undefined): string => {
  switch (type) {
    case 'boolean':
    case 'boolean-group':
      return value ? 'turned on' : 'turned off'

    case 'boolean-enum':
      if (control?.type === 'boolean-enum' && typeof value === 'number' && control.inputs[value]) {
        return `set to ${control.inputs[value].name}`
      }
      break

    case 'enum':
    case 'step-enum':
      if ((control?.type === 'enum' || control?.type === 'step-enum') && typeof value === 'number' && control.options[value]) {
        return `set to ${control.options[value].name}`
      }
      break

    case 'slider':
    case 'intiface-toy':
      if (typeof value === 'number') return `set to ${Math.round(value * 100)}%`
      break

    case 'intiface-pattern':
      if (value === INTIFACE_PATTERN_OFF_ID) return 'turned off'
      if (control?.type === 'intiface-pattern') {
        const pattern = control.allowedPatterns.find((p) => p.id === value)

        if (pattern) return `set to ${pattern.name}`
      }
      break

    case 'open-shock-shocker': {
      const mode = control?.type === 'open-shock-shocker' && control.mode === 'Vibrate' ? 'vibration' : 'shock'

      return isOpenShockCommandValue(value) ? `sent a ${mode} · ${formatCommandValue(value)}` : `sent a ${mode}`
    }

    case 'preset':
      return 'applied it'
  }

  return value === undefined ? 'used it' : `set to ${formatCommandValue(value)}`
}
