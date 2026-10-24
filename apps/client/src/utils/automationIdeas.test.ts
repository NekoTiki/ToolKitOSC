import type { Board } from '@renderer/lib/tauri-bridge'
import { AUTOMATION_IDEAS } from '@renderer/utils/automationIdeas'
import { stepFits, triggerCondition } from '@renderer/utils/automations'
import { describe, expect, it } from 'vitest'

const board = { id: 'b', name: 'Desk', config: [{ pin: 4, label: 'Fan', maxOnMs: 5000 }] } as unknown as Board
const everything = {
  avatarId: 'avtr_1',
  avatarName: 'Rusk',
  controls: [
    { groupId: 'g', id: 'lush', type: 'intiface-toy' as const },
    { groupId: 'g', id: 'collar', type: 'open-shock-shocker' as const }
  ],
  profiles: [{ id: 'stream' }],
  boards: [board]
}

describe('automation ideas', () => {
  it('builds every idea with steps that fit its trigger', () => {
    for (const idea of AUTOMATION_IDEAS) {
      const automation = idea.build(everything)
      expect(automation, idea.id).not.toBeNull()
      const condition = triggerCondition(automation!.when)
      automation!.then.forEach((step) => expect(stepFits(step, condition), idea.id).toBe(true))
    }
  })

  it('leaves out ideas that need what you don\'t have', () => {
    const nothing = { ...everything, controls: [], profiles: [], boards: [] }
    const built = AUTOMATION_IDEAS.filter((idea) => idea.build(nothing)).map((idea) => idea.id)
    expect(built).not.toContain('boop-buzz')
    expect(built).not.toContain('avatar-profile')
    expect(built).not.toContain('board-headpat')
    expect(built).toContain('say-hi')
  })
})
