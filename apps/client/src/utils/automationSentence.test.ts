import type { Automation, AutomationStep } from '@renderer/lib/tauri-bridge'
import { newAutomation } from '@renderer/utils/automations'
import { easyExtras, plainSentence, sentence } from '@renderer/utils/automationSentence'
import { describe, expect, it } from 'vitest'

const automation = (patch: Partial<Automation>): Automation => ({ ...newAutomation('avtr_1', 'Rusk'), name: 'Test', ...patch })

const boop: Automation['when'] = {
  kind: 'avatar-parameter',
  avatarId: 'avtr_1',
  avatarName: 'Rusk',
  parameter: '/avatar/parameters/Boop',
  valueType: 'Bool',
  condition: { type: 'turns-on' },
  anyAvatar: false,
  heldMs: 0
}

const toyPulse = (ms: number, delayMs = 0): Extract<AutomationStep, { kind: 'control' }> => ({
  kind: 'control',
  groupId: 'g',
  controlId: 'lush',
  controlType: 'intiface-toy',
  run: { type: 'pulse', value: 0.6, ms },
  delayMs
})

const context = { boards: [], controlName: (id: string) => (id === 'lush' ? 'Lush 3' : undefined) }

describe('sentence', () => {
  it('reads a moment and a toy pulse as one sentence', () => {
    const s = sentence(automation({ when: boop, then: [toyPulse(2000)], cooldownMs: 5000 }), context)
    expect(plainSentence(s)).toBe('When Boop turns on, buzz Lush 3 at 60% for 2 s, any time, at most every 5 s.')
  })

  it('reads toy steps without a control', () => {
    const lush = { deviceIndex: 0, deviceName: 'Lush 3', actuatorIndex: 0, actuatorDescription: 'Vibrate', actuatorType: 'Vibrate' }
    const s = sentence(
      automation({
        when: boop,
        then: [
          { kind: 'toy', toys: [], run: { type: 'pulse', level: 0.6, ms: 2000 }, delayMs: 0 },
          { kind: 'toy-pattern', toys: [lush], patternId: 'wave', ms: 10_000, whileHeld: false, delayMs: 0 },
          { kind: 'stop-toys', delayMs: 0 }
        ]
      }),
      { ...context, patternName: (id) => (id === 'wave' ? 'Wave' : undefined) }
    )
    expect(plainSentence(s)).toBe(
      'When Boop turns on, buzz all toys at 60% for 2 s, then play Wave on Lush 3 for 10 s, then stop your toys, any time, every time.'
    )
  })

  it('uses "While" for a state and words a hold', () => {
    const s = sentence(
      automation({
        when: { ...boop, condition: { type: 'is-on' }, heldMs: 1000 },
        then: [{ kind: 'avatar-parameter', parameter: '/avatar/parameters/Ears', valueType: 'Bool', set: { type: 'hold', value: 0 }, delayMs: 0 }]
      })
    )
    expect(plainSentence(s)).toBe('While Boop is on for at least 1 s, turn Ears off for as long as it lasts, any time, every time.')
  })

  it('follows a Float and adds a delayed step', () => {
    const s = sentence(
      automation({
        when: { ...boop, parameter: '/avatar/parameters/HeadPat', valueType: 'Float', condition: { type: 'any' } },
        then: [{ ...toyPulse(0), run: { type: 'follow', min: 0, max: 1 } }, { kind: 'chatbox', text: 'pat pat', delayMs: 3000 }]
      }),
      context
    )
    expect(s.lead).toBe('When')
    expect(plainSentence(s)).toBe('When HeadPat changes, buzz Lush 3 following it, then after 3 s say “pat pat” in the chatbox, any time, every time.')
  })

  it('names viewer events and guards', () => {
    const s = sentence(
      automation({
        when: { kind: 'app-event', event: { type: 'viewer-joined' } },
        then: [{ kind: 'chatbox', text: 'Hi!', delayMs: 0 }],
        onlyIf: [{ kind: 'viewers', online: true }, { kind: 'profile', profileId: null }]
      })
    )
    expect(plainSentence(s)).toBe('When a viewer joins, say “Hi!” in the chatbox, only when someone is watching and no profile is on, every time.')
  })

  it('marks steps that still need a target', () => {
    expect(sentence(automation({})).steps[0]!.missing).toBe(true)
  })
})

describe('easyExtras', () => {
  it('is empty for what Easy can show', () => {
    expect(easyExtras(automation({ when: boop, then: [toyPulse(2000, 1000)], cooldownMs: 60_000 }))).toEqual([])
  })

  it('lists odd times, any avatar and repeated pulses', () => {
    const extras = easyExtras(
      automation({
        when: { ...boop, condition: { type: 'is-on' }, heldMs: 1500, anyAvatar: true },
        then: [
          toyPulse(1750, 250),
          { kind: 'board-output', boardId: 'b', pin: 4, output: { type: 'pulse', onMs: 1000, offMs: 500, count: 3 }, delayMs: 0 }
        ],
        cooldownMs: 30_000
      })
    )
    expect(extras).toEqual([
      'Runs on any avatar with this parameter',
      'Has to last 1.5 s first',
      'Step 1 waits 0.25 s',
      'Step 1 lasts 1.75 s',
      'Step 2 repeats 3 times',
      'Waits 30 s between runs'
    ])
  })
})
