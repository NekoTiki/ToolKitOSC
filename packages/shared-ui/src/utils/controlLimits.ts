import type { CommandWithoutIds, ControlLimits, ControlType, ViewerGroupInfo } from '../types/controls'
import { BHAPTICS_EFFECTS, BHAPTICS_LAYOUT, INTIFACE_PATTERN_OFF_ID, OWO_MUSCLES, OWO_SENSATIONS, PATTERN_MAX_MS, PATTERN_MIN_MS, PATTERN_SAMPLE_MS, VIEWER_LIVE_ID, VIEWER_PATTERN_ID } from '../types/controls'

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

// The most generous of several limits: the lowest min, the highest max, and only the options every
// one of them blocks. Any `undefined` (no limits) wins outright.
export const mostGenerousLimits = (list: (ControlLimits | undefined)[]): ControlLimits | undefined => {
  if (!list.length || list.some((limits) => !normalizeLimits(limits))) return undefined

  const ranges = list.map(limitRange)
  const [first, ...rest] = list.map((limits) => limits?.disabledOptions ?? [])

  return normalizeLimits({
    min: Math.min(...ranges.map((range) => range.min)),
    max: Math.max(...ranges.map((range) => range.max)),
    disabledOptions: (first ?? []).filter((key) => rest.every((keys) => keys.includes(key)))
  })
}

// One viewer's copy of a control: the active profile's rules for everyone, opened up by the rules
// of the groups the viewer is in (`groupIds`), the most generous one winning. `groups` names those
// groups, for the share page's badge. The returned control never carries `groupRules`, so it can go
// to a viewer as-is. `unavailable` is left alone: no group can use a toy that's offline.
export function resolveControlForViewer<T extends ControlType>(control: T, groupIds: readonly string[], groups: readonly ViewerGroupInfo[] = []): T {
  const { groupRules, ...base } = control

  delete base.viaGroup

  const everyone = { locked: !!control.locked, limits: normalizeLimits(control.limits) }
  const candidates = groupIds
    .filter((id) => groupRules?.[id])
    .map((id) => {
      const rule = groupRules![id]!

      return { id, locked: rule.locked ?? everyone.locked, limits: rule.limits ? normalizeLimits(rule.limits) : everyone.limits }
    })
    .filter((candidate) => !candidate.locked)

  if (!candidates.length) return base as T

  const open = everyone.locked ? candidates : [{ id: null, ...everyone }, ...candidates]
  const limits = mostGenerousLimits(open.map((candidate) => candidate.limits))
  const changed = everyone.locked || JSON.stringify(limits) !== JSON.stringify(everyone.limits)
  // The first group whose own rule is part of what this viewer gets.
  const via = changed ? groups.find((group) => candidates.some((candidate) => candidate.id === group.id)) : undefined

  return {
    ...base,
    locked: false,
    limits,
    ...(via ? { viaGroup: { name: via.name, color: via.color } } : {})
  } as T
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

      if (command.value === VIEWER_PATTERN_ID || command.value === VIEWER_LIVE_ID) {
        const options = control.viewerPatterns

        if (!options?.enabled) return null

        // Nothing above the control's own cap or the profile's max is ever played.
        const top = Math.min(clamp(options.maxIntensity, 0, 1), limitRange(limits).max)

        if (command.value === VIEWER_LIVE_ID) {
          return isFiniteNumber(command.level) ? { type: command.type, value: command.value, level: clamp(command.level, 0, top) } : null
        }

        const { samples } = command
        const maxSamples = Math.floor(Math.min(options.maxMs, PATTERN_MAX_MS) / PATTERN_SAMPLE_MS)

        if (!Array.isArray(samples) || samples.length * PATTERN_SAMPLE_MS < PATTERN_MIN_MS || samples.length > maxSamples) return null
        if (!samples.every(isFiniteNumber)) return null

        return { type: command.type, value: command.value, samples: samples.map((sample) => Math.round(clamp(sample, 0, top * 100))) }
      }

      const allowed = control.allowedPatterns.some((pattern) => pattern.id === command.value)

      return allowed && !isOptionDisabled(limits, command.value) ? { type: command.type, value: command.value } : null
    }

    case 'open-shock-shocker':
    case 'preset':
      return { type: command.type }

    case 'bhaptics': {
      if (control.type !== 'bhaptics') return null

      // Nothing above the control's own cap or the profile's max is ever played.
      const level = Math.min(clamp(control.maxIntensity, 0, 1), limitRange(limits).max)

      if (command.value === 'effect') {
        const known = BHAPTICS_EFFECTS.some((effect) => effect.id === command.effect)

        return known && control.effects?.includes(command.effect!) ? { type: command.type, value: command.value, effect: command.effect, level } : null
      }
      if (command.value !== 'hold' && command.value !== 'tap') return null
      if (!command.position || !control.positions.includes(command.position)) return null

      const count = BHAPTICS_LAYOUT[command.position].motors
      const { motors } = command

      if (!Array.isArray(motors) || motors.length > count || !motors.every((m) => Number.isInteger(m) && m >= 0 && m < count)) return null
      if (command.value === 'tap' && !motors.length) return null
      if (command.value === 'hold' && !isFiniteNumber(command.holdId)) return null

      const sorted = [...new Set(motors)].sort((a, b) => a - b)

      return command.value === 'tap'
        ? { type: command.type, value: command.value, position: command.position, motors: sorted, level }
        : { type: command.type, value: command.value, position: command.position, motors: sorted, holdId: command.holdId, start: command.start === true, level }
    }

    case 'owo': {
      if (control.type !== 'owo') return null

      const known = OWO_SENSATIONS.some((sensation) => sensation.id === command.value)

      if (!known || !control.sensations.includes(command.value)) return null
      if (!Array.isArray(command.muscles) || !command.muscles.length || !command.muscles.every((m) => control.muscles.includes(m))) return null

      // Nothing above the control's own cap or the profile's max is ever played.
      const level = Math.min(clamp(control.maxIntensity, 0, 1), limitRange(limits).max)
      // In OWO's order, each once.
      const muscles = (Object.keys(OWO_MUSCLES) as (keyof typeof OWO_MUSCLES)[]).filter((m) => command.muscles.includes(m))

      return { type: command.type, value: command.value, muscles, level }
    }

    default:
      return null
  }
}
