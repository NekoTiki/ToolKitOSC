import type { IntifaceActuatorRef } from '@toolkitosc/shared-ui'
import { INTIFACE_PATTERN_OFF_ID, useIntifacePatternControl } from '@toolkitosc/shared-ui'

import { useIntiface } from './useIntiface'

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
}

// Deterministic PRNG so 'Random' doesn't need to carry mutable state between calls - the same
// elapsed-time step always maps to the same value within that step's window.
const seededRandom = (seed: number): number => {
  const x = Math.sin(seed * 12.9898) * 43758.5453
  return x - Math.floor(x)
}

// Built-in patterns only, for now - this registry is deliberately the one and only place that
// knows how to turn a pattern id into an actual waveform. A control only ever stores a pattern's
// id/name/icon (see IntifacePatternRef in controls.ts), denormalized at authoring time the same
// way a toy's name is - so a future custom/user-authored pattern just needs its own
// IntifacePatternDefinition added to a registry (this one, or a second user-editable one merged
// alongside it); nothing about the control data, the settings picker, or playback below needs to
// change to accommodate it.
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
    value: (t) => {
      const stepMs = 350

      return seededRandom(Math.floor(t / stepMs))
    }
  }
]

export const getPattern = (id: string): IntifacePatternDefinition | undefined =>
  BUILTIN_PATTERNS.find((pattern) => pattern.id === id)

const PLAYBACK_INTERVAL_MS = 50

interface RunningPattern {
  actuators: IntifaceActuatorRef[]
  startedAt: number
  timer: ReturnType<typeof setInterval>
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
  stopPattern: (controlId: string) => void
  // Stops any running pattern (other than `exceptControlId`) that owns any of `actuators` - call
  // this whenever something else is about to drive one of these actuators directly (a plain toy
  // slider, most notably), so a manually-adjusted toy doesn't keep fighting a pattern still running
  // underneath it.
  releaseActuators: (actuators: IntifaceActuatorRef[], exceptControlId?: string) => void
} {
  const { setActuatorIntensity } = useIntiface()
  const { setValue: setPatternValue } = useIntifacePatternControl()

  const stopPattern = (controlId: string): void => {
    const entry = runningPatterns.get(controlId)

    if (!entry) return

    clearInterval(entry.timer)
    runningPatterns.delete(controlId)
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

    if (!pattern) return

    releaseActuators(actuators, controlId)

    const startedAt = Date.now()

    const tick = (): void => {
      setActuatorIntensity(actuators, pattern.value(Date.now() - startedAt))
    }

    tick()

    const timer = setInterval(tick, PLAYBACK_INTERVAL_MS)

    runningPatterns.set(controlId, { actuators, startedAt, timer })
    setPatternValue(controlId, patternId)
  }

  return { playPattern, stopPattern, releaseActuators }
}
