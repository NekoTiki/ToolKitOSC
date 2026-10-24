import { describe, expect, it } from 'vitest'

import { accessDecision } from '../server/utils/shareAccess'

describe('accessDecision', () => {
  it('lets everyone in when the host lets anyone in', () => {
    expect(accessDecision(undefined, undefined)).toBe('admit')
    expect(accessDecision({ approval: false }, undefined)).toBe('admit')
  })

  it('asks a guest to sign in when the host approves people', () => {
    expect(accessDecision({ approval: true, approved: ['1'] }, undefined)).toBe('sign-in')
  })

  it('lets an approved account in and keeps others waiting', () => {
    expect(accessDecision({ approval: true, approved: ['1'] }, '1')).toBe('admit')
    expect(accessDecision({ approval: true, approved: ['1'] }, '2')).toBe('wait')
  })

  it('keeps everyone waiting while the approved list is unknown', () => {
    expect(accessDecision({ approval: true }, '1')).toBe('wait')
  })
})
