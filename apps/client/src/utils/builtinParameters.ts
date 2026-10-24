import type { AutomationGuard, AvatarParameterTrigger } from '@renderer/lib/tauri-bridge'

// VRChat's own parameters, the same on every avatar (docs.vrchat.com/docs/animator-parameters).
// Easy mode offers them as "a gesture" and "you": they're saved as ordinary avatar parameter
// triggers and checks, on any avatar.

const address = (name: string): string => `/avatar/parameters/${name}`

// GestureLeft / GestureRight values 0-7.
export const GESTURES = ['Neutral', 'Fist', 'Open hand', 'Point', 'Victory', 'Rock and roll', 'Finger gun', 'Thumbs up']

export type Hand = 'left' | 'right'
export const HAND_ADDRESS: Record<Hand, string> = { left: address('GestureLeft'), right: address('GestureRight') }

export const handOf = (parameter: string): Hand | null =>
  parameter === HAND_ADDRESS.left ? 'left' : parameter === HAND_ADDRESS.right ? 'right' : null

// Bools that say what you're doing, worded for the sentence ("you go AFK", "while you're AFK").
export interface YouState {
  address: string
  label: string
  is: string
  on: string
  off: string
}

export const YOU_STATES: YouState[] = [
  { address: address('AFK'), label: 'AFK', is: 'AFK', on: 'go AFK', off: 'come back from AFK' },
  { address: address('Seated'), label: 'Seated', is: 'seated', on: 'sit down', off: 'stand up' },
  { address: address('MuteSelf'), label: 'Muted', is: 'muted', on: 'mute yourself', off: 'unmute' },
  { address: address('InStation'), label: 'In a station', is: 'in a station', on: 'get in a station', off: 'leave a station' }
]

// An Int, 1 in VR and 0 on desktop: a check only.
export const VR_MODE = address('VRMode')

export const youState = (parameter: string): YouState | undefined => YOU_STATES.find((s) => s.address === parameter)

export const isBuiltin = (parameter: string): boolean => !!handOf(parameter) || !!youState(parameter) || parameter === VR_MODE

export const gestureTrigger = (avatarId: string, avatarName: string, hand: Hand, gesture: number): AvatarParameterTrigger => ({
  kind: 'avatar-parameter',
  avatarId,
  avatarName,
  parameter: HAND_ADDRESS[hand],
  valueType: 'Int',
  condition: { type: 'becomes', value: gesture },
  anyAvatar: true,
  heldMs: 0
})

export const youTrigger = (avatarId: string, avatarName: string, state: YouState): AvatarParameterTrigger => ({
  kind: 'avatar-parameter',
  avatarId,
  avatarName,
  parameter: state.address,
  valueType: 'Bool',
  condition: { type: 'turns-on' },
  anyAvatar: true,
  heldMs: 0
})

// "you're AFK", "you're not muted", "you're in VR": `yes` false turns the check around.
export const youGuard = (parameter: string, yes: boolean): AutomationGuard =>
  parameter === VR_MODE
    ? { kind: 'parameter', parameter, valueType: 'Int', condition: { type: 'is', value: 1 }, not: !yes }
    : { kind: 'parameter', parameter, valueType: 'Bool', condition: { type: 'is-on' }, not: !yes }

// The sentence's words for a trigger on a built-in, or null for any other parameter.
export const builtinWhenText = (when: AvatarParameterTrigger): { before: string; name: string } | null => {
  const hand = handOf(when.parameter)
  if (hand && 'value' in when.condition) {
    return { before: `your ${hand} hand makes `, name: GESTURES[when.condition.value] ?? `gesture ${when.condition.value}` }
  }
  const state = youState(when.parameter)
  if (!state) return null
  switch (when.condition.type) {
    case 'turns-on':
      return { before: 'you ', name: state.on }
    case 'turns-off':
      return { before: 'you ', name: state.off }
    case 'is-on':
      return { before: "you're ", name: state.is }
    default:
      return null
  }
}

// The sentence's words for a check on a built-in, or null for any other parameter.
export const builtinGuardText = (guard: AutomationGuard): string | null => {
  if (guard.kind !== 'parameter') return null
  if (guard.parameter === VR_MODE) return guard.not ? 'on desktop' : 'in VR'
  const state = youState(guard.parameter)
  if (!state || guard.condition.type !== 'is-on') return null
  return guard.not ? `not ${state.is}` : state.is
}
