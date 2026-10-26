import { describe, expect, it } from 'vitest'

import { isValidShareName } from './shortLink'

describe('isValidShareName', () => {
  it('accepts lowercase names with inner dashes', () => {
    for (const name of ['abc', 'neko-tiki', 'a1b2', 'x'.repeat(24)]) expect(isValidShareName(name)).toBe(true)
  })

  it('rejects bad lengths, characters, edge dashes and reserved names', () => {
    for (const name of ['ab', 'x'.repeat(25), 'Neko', 'neko_tiki', 'neko tiki', '-neko', 'neko-', 'admin', 'toolkitosc']) {
      expect(isValidShareName(name)).toBe(false)
    }
  })
})
