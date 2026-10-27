import { describe, expect, it, vi } from 'vitest'

import { normalizeSuggestion } from '../server/utils/ai/normalize'

// Nuxt's ~~ alias doesn't resolve under plain vitest, and normalize never reads the settings.
vi.mock('~~/server/utils/serverSettings', () => ({ getSettings: () => ({}) }))

const parameters = [
  { name: 'QuickChange', address: '/avatar/parameters/QuickChange', kind: 'Bool' as const },
  { name: 'Size', address: '/avatar/parameters/Size', kind: 'Float' as const }
]

const first = (suggestion: object) =>
  normalizeSuggestion({ groups: [{ name: 'Outfit', controls: [{ name: 'Quick change', ...suggestion } as never] }] }, parameters)[0]?.controls[0]

describe('normalizeSuggestion buttons', () => {
  it('makes a button from a Bool the model picked it for', () => {
    expect(first({ type: 'button', address: '/avatar/parameters/QuickChange' })).toMatchObject({ type: 'button', inputAddress: '/avatar/parameters/QuickChange' })
  })

  it('keeps a plain toggle a toggle', () => {
    expect(first({ type: 'boolean', address: '/avatar/parameters/QuickChange' })).toMatchObject({ type: 'boolean' })
  })

  it('never makes a button from a parameter that is not a Bool', () => {
    expect(first({ type: 'button', address: '/avatar/parameters/Size' })).toMatchObject({ type: 'slider' })
  })
})
