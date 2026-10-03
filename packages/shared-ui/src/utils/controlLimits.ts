import type { CommandWithoutIds, ControlLimits, ControlType } from '../types/controls'
import { INTIFACE_PATTERN_OFF_ID } from '../types/controls'

const clamp = (value: number, min: number, max: number): number => Math.min(max, Math.max(min, value))

export const isOptionDisabled = (limits: ControlLimits | undefined, key: string | number): boolean =>
  !!limits?.disabledOptions?.includes(key)

// The range a viewer may set on a Slider or Toy, 0-1.
export const limitRange = (limits: ControlLimits | undefined): { min: number; max: number } => {
  const min = clamp(limits?.min ?? 0, 0, 1)

  return { min, max: clamp(limits?.max ?? 1, min, 1) }
}

// Drops the parts of a ControlLimits that don't limit anything (full range, nothing disabled), so
// "no limits" is always stored and sent as `undefined`.
export const normalizeLimits = (limits: ControlLimits | undefined): ControlLimits | undefined => {
  if (!limits) return undefined

  const normalized: ControlLimits = {}

  if (limits.min !== undefined && limits.min > 0) normalized.min = clamp(limits.min, 0, 1)
  if (limits.max !== undefined && limits.max < 1) normalized.max = clamp(limits.max, normalized.min ?? 0, 1)
  if (limits.disabledOptions?.length) normalized.disabledOptions = [...limits.disabledOptions]

  return Object.keys(normalized).length ? normalized : undefined
}

const isFiniteNumber = (value: unknown): value is number => typeof value === 'number' && Number.isFinite(value)

// Checks a viewer's command against the control it targets, as the viewer page last saw it
// (locked/unavailable flags and the active profile's limits included). Returns the command to run -
// slider and toy values clamped into range - or null when it must be dropped. Viewer input is never
// trusted: a hand-written WebSocket message can carry any value, so this runs on the relay and
// again on the host, which has the final say.
export function sanitizeCommand(control: ControlType, command: CommandWithoutIds): CommandWithoutIds | null {
  if (!command || command.type !== control.type || control.locked || control.unavailable) return null

  const { limits } = control

  switch (command.type) {
    case 'boolean':
    case 'boolean-group':
      return typeof command.value === 'boolean' ? { type: command.type, value: command.value } : null

    case 'boolean-enum': {
      if (control.type !== 'boolean-enum' || !Number.isInteger(command.value)) return null

      const input = control.inputs[command.value]

      return input && !isOptionDisabled(limits, input.id) ? { type: command.type, value: command.value } : null
    }

    case 'enum':
    case 'step-enum': {
      if ((control.type !== 'enum' && control.type !== 'step-enum') || !Number.isInteger(command.value)) return null

      const option = control.options[command.value]

      return option && !isOptionDisabled(limits, option.value) ? { type: command.type, value: command.value } : null
    }

    case 'slider':
    case 'intiface-toy': {
      if (!isFiniteNumber(command.value)) return null

      const { min, max } = limitRange(limits)

      return { type: command.type, value: clamp(command.value, min, max) }
    }

    case 'intiface-pattern': {
      if (control.type !== 'intiface-pattern' || typeof command.value !== 'string') return null
      if (command.value === INTIFACE_PATTERN_OFF_ID) return { type: command.type, value: command.value }

      const allowed = control.allowedPatterns.some((pattern) => pattern.id === command.value)

      return allowed && !isOptionDisabled(limits, command.value) ? { type: command.type, value: command.value } : null
    }

    case 'open-shock-shocker':
    case 'preset':
      return { type: command.type }

    default:
      return null
  }
}
