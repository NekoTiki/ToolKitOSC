import type { IntifaceActuatorRef, IntifacePatternRef } from '@toolkitosc/shared-ui'
import { INTIFACE_PATTERN_OFF_ID, useIntifacePatternControl } from '@toolkitosc/shared-ui'

import { useIntiface } from './useIntiface'
import type { SavedPattern } from './useSavedPatterns'
import { PATTERN_SAMPLE_MS, useSavedPatterns } from './useSavedPatterns'

// Buttplug/Intiface has no server-side concept of a named, timed pattern (Pulse/Wave/...) - every
// message is a one-shot "set this value now" instruction (see the ScalarCmd usage in
// useIntiface.ts). Patterns are entirely a client-side thing: this repeatedly samples a pattern's
// `value` function on an interval and sends whatever comes out, same as a human dragging the plain
// toy slider very quickly.
export interface IntifacePatternDefinition {
  id: string
  name: string
  icon?: string
  // Scalar value (0-1) this pattern wants at `elapsedMs` since it started playing.
  value: (elapsedMs: number) => number
  // Set for a pattern the host recorded (see useSavedPatterns.ts): the length of one loop.
  durationMs?: number
}

// Deterministic PRNG so 'Random' doesn't need to carry mutable state between calls - the same
// elapsed-time step always maps to the same value within that step's window.
const seededRandom = (seed: number): number => {
  const x = Math.sin(seed * 12.9898) * 43758.5453
  return x - Math.floor(x)
}

// Smooth noise: a random level every `stepMs`, eased from one to the next instead of jumping.
const smoothNoise = (t: number, stepMs: number, seedOffset = 0): number => {
  const position = t / stepMs
  const step = Math.floor(position)
  const f = position - step
  const eased = f * f * (3 - 2 * f)
  const from = seededRandom(step + seedOffset)

  return from + (seededRandom(step + 1 + seedOffset) - from) * eased
}

// The one place that knows how to turn a pattern id into an actual waveform. A control only ever
// stores a pattern's id/name/icon (see IntifacePatternRef in controls.ts), denormalized at authoring
// time the same way a toy's name is. Two registries feed it: the built-in patterns below and the
// ones the host recorded (useSavedPatterns.ts), which getPattern looks up first.
export const BUILTIN_PATTERNS: IntifacePatternDefinition[] = [
  {
    id: 'pulse',
    name: 'Pulse',
    icon: 'i-lucide-heart-pulse',
    value: (t) => (Math.sin(t / 250) + 1) / 2
  },
  {
    id: 'wave',
    name: 'Wave',
    icon: 'i-lucide-waves',
    value: (t) => (Math.sin(t / 2000) + 1) / 2
  },
  {
    id: 'escalate',
    name: 'Escalate',
    icon: 'i-lucide-trending-up',
    value: (t) => {
      const cycleMs = 8000

      return (t % cycleMs) / cycleMs
    }
  },
  {
    id: 'random',
    name: 'Random',
    icon: 'i-lucide-shuffle',
    // A slow drift with a little faster wobble on top, kept off the extremes, so it wanders instead
    // of jumping between full random levels.
    value: (t) => 0.15 + 0.75 * (0.75 * smoothNoise(t, 1200) + 0.25 * smoothNoise(t, 400, 1000))
  }
]

// A recording loops its samples, one every PATTERN_SAMPLE_MS.
export const toDefinition = (pattern: Pick<SavedPattern, 'id' | 'name' | 'icon' | 'samples'>): IntifacePatternDefinition => ({
  id: pattern.id,
  name: pattern.name,
  icon: pattern.icon,
  durationMs: pattern.samples.length * PATTERN_SAMPLE_MS,
  value: (t) => (pattern.samples[Math.floor(t / PATTERN_SAMPLE_MS) % pattern.samples.length] ?? 0) / 100
})

const { savedPatterns, getSavedPattern } = useSavedPatterns()

// Every pattern a control can offer: the host's own first, then the built-in ones.
export const allPatterns = (): IntifacePatternDefinition[] => [...savedPatterns.value.map(toDefinition), ...BUILTIN_PATTERNS]

export const getPattern = (id: string): IntifacePatternDefinition | undefined => {
  const saved = getSavedPattern(id)

  return saved ? toDefinition(saved) : BUILTIN_PATTERNS.find((pattern) => pattern.id === id)
}

// A control's copy of each pattern's name and icon goes stale when the host renames a recording,
// and points at nothing once it's deleted. This refreshes the copies and drops the missing ones.
export const resolvePatternRefs = (refs: IntifacePatternRef[]): IntifacePatternRef[] =>
  refs.flatMap((ref) => {
    const pattern = getPattern(ref.id)

    return pattern ? [{ id: pattern.id, name: pattern.name, icon: pattern.icon }] : []
  })

const PLAYBACK_INTERVAL_MS = 50

interface RunningPattern {
  actuators: IntifaceActuatorRef[]
  startedAt: number
  timer: ReturnType<typeof setInterval>
  // Ends the pattern by itself (see PlayOptions.stopAfterMs).
  stopTimer?: ReturnType<typeof setTimeout>
  onEnd?: () => void
}

export interface PlayOptions {
  // Stops the pattern (and zeroes its toys) after this long. A viewer's own pattern uses it.
  stopAfterMs?: number | null
  // Who started it, broadcast with the pattern value so the share page can say "Sam's pattern".
  by?: string
  // Called once when it stops, whatever stops it (its timer, another pattern, Stop everything...).
  onEnd?: () => void
}

// Module-scope singleton, same reasoning as useIntiface.ts's own connection state: every control
// instance (the real rendered one, a live authoring preview, ...) needs to see and stop the same
// timer for a given control id, not each think they own a separate one.
const runningPatterns = new Map<string, RunningPattern>()

const actuatorKey = (actuator: Pick<IntifaceActuatorRef, 'deviceIndex' | 'actuatorIndex'>): string =>
  `${actuator.deviceIndex}:${actuator.actuatorIndex}`

export function useIntifacePatterns(): {
  // Starts (or stops, for INTIFACE_PATTERN_OFF_ID) the given pattern against `actuators` for this
  // control. Claims those actuators first (see releaseActuators) - only one thing should ever be
  // driving a given actuator's value at a time, whether that's another running pattern or this one
  // replacing its own previous pattern.
  playPattern: (controlId: string, patternId: string, actuators: IntifaceActuatorRef[]) => void
  // Same, for a pattern that isn't in a registry (yet): the Patterns page tries out an unsaved take.
  playDefinition: (controlId: string, pattern: IntifacePatternDefinition, actuators: IntifaceActuatorRef[], options?: PlayOptions) => void
  stopPattern: (controlId: string) => void
  // Stops any running pattern (other than `exceptControlId`) that owns any of `actuators` - call
  // this whenever something else is about to drive one of these actuators directly (a plain toy
  // slider, most notably), so a manually-adjusted toy doesn't keep fighting a pattern still running
  // underneath it.
  releaseActuators: (actuators: IntifaceActuatorRef[], exceptControlId?: string) => void
  // Stops every running pattern and zeroes its toys, whichever avatar's control started it.
  stopAllPatterns: () => void
} {
  const { setActuatorIntensity } = useIntiface()
  const { setValue: setPatternValue } = useIntifacePatternControl()

  const stopPattern = (controlId: string): void => {
    const entry = runningPatterns.get(controlId)

    if (!entry) return

    clearInterval(entry.timer)
    clearTimeout(entry.stopTimer)
    runningPatterns.delete(controlId)
    entry.onEnd?.()
  }

  const releaseActuators = (actuators: IntifaceActuatorRef[], exceptControlId?: string): void => {
    const keys = new Set(actuators.map(actuatorKey))

    Array.from(runningPatterns.keys()).forEach((controlId) => {
      if (controlId === exceptControlId) return

      const entry = runningPatterns.get(controlId)

      if (!entry?.actuators.some((actuator) => keys.has(actuatorKey(actuator)))) return

      stopPattern(controlId)
      setPatternValue(controlId, INTIFACE_PATTERN_OFF_ID)
    })
  }

  const playPattern = (
    controlId: string,
    patternId: string,
    actuators: IntifaceActuatorRef[]
  ): void => {
    stopPattern(controlId)

    if (patternId === INTIFACE_PATTERN_OFF_ID) {
      // Stopping the interval alone leaves the toy sitting at whatever value the last tick before
      // this happened to land on - explicitly zero it out so switching to Off actually stops it.
      setActuatorIntensity(actuators, 0)
      setPatternValue(controlId, INTIFACE_PATTERN_OFF_ID)
      return
    }

    const pattern = getPattern(patternId)

    if (pattern) playDefinition(controlId, pattern, actuators)
  }

  const playDefinition = (
    controlId: string,
    pattern: IntifacePatternDefinition,
    actuators: IntifaceActuatorRef[],
    options: PlayOptions = {}
  ): void => {
    stopPattern(controlId)
    releaseActuators(actuators, controlId)

    const startedAt = Date.now()

    const tick = (): void => {
      setActuatorIntensity(actuators, pattern.value(Date.now() - startedAt))
    }

    tick()

    const timer = setInterval(tick, PLAYBACK_INTERVAL_MS)
    const { stopAfterMs, by, onEnd } = options
    const stopTimer = stopAfterMs ? setTimeout(() => playPattern(controlId, INTIFACE_PATTERN_OFF_ID, actuators), stopAfterMs) : undefined

    runningPatterns.set(controlId, { actuators, startedAt, timer, stopTimer, onEnd })
    setPatternValue(controlId, pattern.id, by || stopTimer ? { by, endsAt: stopAfterMs ? startedAt + stopAfterMs : undefined } : undefined)
  }

  const stopAllPatterns = (): void => {
    Array.from(runningPatterns.entries()).forEach(([controlId, entry]) => {
      stopPattern(controlId)
      setActuatorIntensity(entry.actuators, 0)
      setPatternValue(controlId, INTIFACE_PATTERN_OFF_ID)
    })
  }

  return { playPattern, playDefinition, stopPattern, releaseActuators, stopAllPatterns }
}
