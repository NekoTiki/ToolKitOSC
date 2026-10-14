import type { ToyActuatorRef } from '@renderer/lib/tauri-bridge'
import { api } from '@renderer/lib/tauri-bridge'
import type { IntifaceActuatorRef, IntifacePatternRef } from '@toolkitosc/shared-ui'
import { INTIFACE_PATTERN_OFF_ID, useIntifacePatternControl } from '@toolkitosc/shared-ui'

import type { SavedPattern } from './useSavedPatterns'
import { PATTERN_SAMPLE_MS, useSavedPatterns } from './useSavedPatterns'

// Buttplug/Intiface has no server-side concept of a named, timed pattern (Pulse/Wave/...) - every
// message is a one-shot "set this value now" instruction. Patterns are played by sampling a level
// every 50 ms, which Rust does (src-tauri/src/toys/patterns.rs, with the same maths as below) so a
// hidden window can't slow it. The definitions here are for showing patterns.
export interface IntifacePatternDefinition {
  id: string
  name: string
  icon?: string
  // Scalar value (0-1) this pattern wants at `elapsedMs` since it started playing.
  value: (elapsedMs: number) => number
  // Set for a pattern the host recorded (see useSavedPatterns.ts): the length of one loop.
  durationMs?: number
  // Set for a recording: what Rust plays. Without it, Rust plays the pattern by its id.
  samples?: number[]
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
  samples: pattern.samples,
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

export interface PlayOptions {
  // Stops the pattern (and zeroes its toys) after this long. A viewer's own pattern uses it.
  stopAfterMs?: number | null
  // Who started it, broadcast with the pattern value so the share page can say "Sam's pattern".
  by?: string
  // Called once when it stops, whatever stops it (its timer, another pattern, Stop everything...).
  onEnd?: () => void
}

// Only the fields Rust needs: a control's actuator refs also carry names.
const toRefs = (actuators: IntifaceActuatorRef[]): ToyActuatorRef[] =>
  actuators.map(({ deviceIndex, actuatorIndex, actuatorType }) => ({ deviceIndex, actuatorIndex, actuatorType }))

const { setValue: setPatternValue } = useIntifacePatternControl()

// Rust reports every change to a control's pattern, including the ones it makes by itself (a
// viewer's pattern timing out, another pattern taking the toy over).
api.onPatternValue(({ controlId, value, info }) => setPatternValue(controlId, value, info ?? undefined))

// onEnd callbacks by run number, and runs that ended before their number came back.
const endCallbacks = new Map<number, () => void>()
const endedRuns = new Set<number>()

api.onPatternEnded(({ run }) => {
  const onEnd = endCallbacks.get(run)

  endCallbacks.delete(run)
  if (onEnd) onEnd()
  else endedRuns.add(run)
})

// Calls `onEnd` once pattern run `run` stops, even if it already has.
export const onPatternRunEnd = (run: number, onEnd: () => void): void => {
  if (endedRuns.delete(run)) onEnd()
  else endCallbacks.set(run, onEnd)
}

export function useIntifacePatterns(): {
  // Starts (or stops, for INTIFACE_PATTERN_OFF_ID) the given pattern against `actuators` for this
  // control. Claims those actuators first (see releaseActuators) - only one thing should ever be
  // driving a given actuator's value at a time.
  playPattern: (controlId: string, patternId: string, actuators: IntifaceActuatorRef[]) => void
  // Same, for a pattern that isn't in a registry (yet): the Patterns page tries out an unsaved take.
  playDefinition: (controlId: string, pattern: IntifacePatternDefinition, actuators: IntifaceActuatorRef[], options?: PlayOptions) => void
  stopPattern: (controlId: string) => void
  // Stops any running pattern (other than `exceptControlId`) that owns any of `actuators` - call
  // this whenever something else is about to drive one of these actuators directly (a plain toy
  // slider, most notably), so a manually-adjusted toy doesn't keep fighting a pattern underneath it.
  releaseActuators: (actuators: IntifaceActuatorRef[], exceptControlId?: string) => void
  // Stops every running pattern and zeroes its toys, whichever avatar's control started it.
  stopAllPatterns: () => void
} {
  const playPattern = (controlId: string, patternId: string, actuators: IntifaceActuatorRef[]): void => {
    if (patternId === INTIFACE_PATTERN_OFF_ID) void api.patternsOff(controlId, toRefs(actuators))
    else void api.patternsPlay(controlId, { id: patternId }, toRefs(actuators))
  }

  const playDefinition = (
    controlId: string,
    pattern: IntifacePatternDefinition,
    actuators: IntifaceActuatorRef[],
    { stopAfterMs, by, onEnd }: PlayOptions = {}
  ): void => {
    void api.patternsPlay(controlId, { id: pattern.id, samples: pattern.samples }, toRefs(actuators), stopAfterMs, by).then((run) => {
      if (run !== null && onEnd) onPatternRunEnd(run, onEnd)
    })
  }

  return {
    playPattern,
    playDefinition,
    stopPattern: (controlId) => void api.patternsStop(controlId),
    releaseActuators: (actuators, exceptControlId) => void api.patternsRelease(toRefs(actuators), exceptControlId),
    stopAllPatterns: () => void api.patternsStopAll()
  }
}
