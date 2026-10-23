import type { Automation, AutomationStep, AutomationTrigger, AvatarParameterTrigger, Board, ParameterCondition, ParameterValueType } from '@renderer/lib/tauri-bridge'
import { newAutomation, newStep } from '@renderer/utils/automations'
import type { ControlTypes } from '@toolkitosc/shared-ui'

// Ready-made automations to start from ("Start from an idea" on the Automations page). Each one
// is built with what you have (your first toy, profile, board output...), and isn't offered when
// it needs something you don't have or an add-on that's off. Ideas that start from your avatar
// leave the parameter empty, so the editor opens on it with Listen ready.

export interface IdeaContext {
  avatarId: string
  avatarName: string
  // Your controls whose add-on is on.
  controls: { groupId: string; id: string; type: ControlTypes }[]
  profiles: { id: string }[]
  boards: Board[]
}

export interface AutomationIdea {
  id: string
  title: string
  description: string
  // What starts it, and what it does.
  icons: [string, string]
  // The trigger and step kinds it uses: it's left out while an add-on they need is off.
  kinds: string[]
  // Null when you don't have what it needs.
  build: (context: IdeaContext) => Automation | null
}

const step = <K extends AutomationStep['kind']>(kind: K, patch: Partial<Extract<AutomationStep, { kind: K }>>): AutomationStep =>
  ({ ...newStep(kind, 'turns-on'), ...patch }) as AutomationStep

const automation = (context: IdeaContext, name: string, when: AutomationTrigger | null, then: AutomationStep[], patch: Partial<Automation> = {}): Automation => {
  const base = newAutomation(context.avatarId, context.avatarName)
  return { ...base, name, when: when ?? base.when, then, ...patch }
}

// Your avatar, from a parameter you pick (with Listen) once the editor opens.
const avatarWhen = (context: IdeaContext, valueType: ParameterValueType, condition: ParameterCondition): AvatarParameterTrigger => ({
  kind: 'avatar-parameter',
  avatarId: context.avatarId,
  avatarName: context.avatarName,
  parameter: '',
  valueType,
  condition,
  anyAvatar: false,
  heldMs: 0
})

const firstControl = (context: IdeaContext, type: ControlTypes) => context.controls.find((c) => c.type === type)

export const AUTOMATION_IDEAS: AutomationIdea[] = [
  {
    id: 'boop-buzz',
    title: 'Boop buzz',
    description: 'A boop buzzes your toy for 2 seconds.',
    icons: ['i-lucide-person-standing', 'i-lucide-vibrate'],
    kinds: ['avatar-parameter', 'control'],
    build: (context) => {
      const toy = firstControl(context, 'intiface-toy')
      if (!toy) return null
      return automation(context, 'Boop buzz', null, [
        step('control', { groupId: toy.groupId, controlId: toy.id, controlType: toy.type, run: { type: 'pulse', value: 0.6, ms: 2000 } })
      ], { cooldownMs: 5000 })
    }
  },
  {
    id: 'headpat-follows',
    title: 'Headpat follows',
    description: 'The closer the headpat, the stronger your toy.',
    icons: ['i-lucide-hand', 'i-lucide-vibrate'],
    kinds: ['avatar-parameter', 'control'],
    build: (context) => {
      const toy = firstControl(context, 'intiface-toy')
      if (!toy) return null
      return automation(context, 'Headpat follows', avatarWhen(context, 'Float', { type: 'any' }), [
        step('control', { groupId: toy.groupId, controlId: toy.id, controlType: toy.type, run: { type: 'follow', min: 0, max: 1 } })
      ])
    }
  },
  {
    id: 'say-hi',
    title: 'Say hi',
    description: 'Greets viewers in your chatbox when they join.',
    icons: ['i-lucide-users', 'i-lucide-message-square'],
    kinds: ['app-event', 'chatbox'],
    build: (context) =>
      automation(context, 'Say hi', { kind: 'app-event', event: { type: 'viewer-joined' } }, [step('chatbox', { text: 'Hi, welcome in!' })], {
        cooldownMs: 60_000,
        retrigger: 'ignore'
      })
  },
  {
    id: 'thank-shocks',
    title: 'Thank for shocks',
    description: 'Says so in your chatbox when a viewer shocks you.',
    icons: ['i-lucide-zap', 'i-lucide-message-square'],
    kinds: ['app-event', 'chatbox'],
    build: (context) =>
      firstControl(context, 'open-shock-shocker')
        ? automation(
            context,
            'Thank for shocks',
            { kind: 'app-event', event: { type: 'viewer-command', controlId: null, controlType: 'open-shock-shocker' } },
            [step('chatbox', { text: 'Ouch! Thank you' })],
            { cooldownMs: 5000 }
          )
        : null
  },
  {
    id: 'stop-when-alone',
    title: 'Stop when everyone leaves',
    description: 'Stops everything once nobody is on your share page.',
    icons: ['i-lucide-user-x', 'i-lucide-octagon-x'],
    kinds: ['app-event', 'stop-everything'],
    build: (context) => automation(context, 'Stop when everyone leaves', { kind: 'app-event', event: { type: 'last-viewer-left' } }, [step('stop-everything', {})])
  },
  {
    id: 'avatar-profile',
    title: 'Profile for this avatar',
    description: 'Switches profile when you put on this avatar.',
    icons: ['i-lucide-repeat', 'i-lucide-layers'],
    kinds: ['avatar-changed', 'profile'],
    build: (context) =>
      context.profiles[0]
        ? automation(context, `${context.avatarName} profile`, { kind: 'avatar-changed', avatarId: context.avatarId, avatarName: context.avatarName }, [
            step('profile', { profileId: context.profiles[0].id, whileHeld: false })
          ])
        : null
  },
  {
    id: 'leash-chat',
    title: 'Leashed',
    description: 'Says so in your chatbox when someone grabs your leash.',
    icons: ['i-tkosc-collar', 'i-lucide-message-square'],
    kinds: ['leash', 'chatbox'],
    build: (context) => automation(context, 'Leashed', { kind: 'leash', event: 'grabbed' }, [step('chatbox', { text: 'Leashed!' })], { cooldownMs: 5000 })
  },
  {
    id: 'grab-heartbeat',
    title: 'Heartbeat when grabbed',
    description: 'Plays a heartbeat on your bHaptics vest when someone grabs you.',
    icons: ['i-lucide-person-standing', 'i-lucide-heart-pulse'],
    kinds: ['avatar-parameter', 'bhaptics-effect'],
    build: (context) => automation(context, 'Heartbeat when grabbed', null, [step('bhaptics-effect', {})], { cooldownMs: 5000 })
  },
  {
    id: 'boop-tap',
    title: 'Boop tap',
    description: 'A boop taps your chest through your OWO Skin.',
    icons: ['i-lucide-person-standing', 'i-lucide-shirt'],
    kinds: ['avatar-parameter', 'owo-sensation'],
    build: (context) => automation(context, 'Boop tap', null, [step('owo-sensation', {})], { cooldownMs: 5000 })
  },
  {
    id: 'board-headpat',
    title: 'Fan on headpat',
    description: 'Something wired to your ESP32 runs while you get headpats.',
    icons: ['i-lucide-hand', 'i-lucide-fan'],
    kinds: ['avatar-parameter', 'board-output'],
    build: (context) => {
      const board = context.boards.find((b) => b.config?.length)
      if (!board) return null
      return automation(context, 'Fan on headpat', avatarWhen(context, 'Bool', { type: 'is-on' }), [
        step('board-output', { boardId: board.id, pin: board.config![0]!.pin, output: { type: 'hold' } })
      ])
    }
  }
]
