import { describe, expect, it } from 'vitest'

import { parseAddonStats } from '../server/utils/addonStats'

describe('parseAddonStats', () => {
  it('keeps ids, switches and device counts', () => {
    expect(
      parseAddonStats({
        addons: [
          { id: 'openshock', enabled: true, devices: 3 },
          { id: 'leash', enabled: true, devices: null },
          { id: 'intiface', enabled: false, devices: null }
        ]
      })
    ).toEqual([
      { addonId: 'openshock', enabled: true, devices: 3 },
      { addonId: 'leash', enabled: true, devices: null },
      { addonId: 'intiface', enabled: false, devices: null }
    ])
  })

  it('drops malformed entries and clamps counts', () => {
    expect(
      parseAddonStats({
        addons: [
          { id: 'Bad Id!', enabled: true },
          { id: 'boards', enabled: 'yes' },
          { id: 'owo', enabled: true, devices: -1 },
          { id: 'bhaptics', enabled: true, devices: 1.5 },
          { id: 'intiface', enabled: true, devices: 1e9 },
          { id: 'openshock', enabled: false, devices: 4 },
          null
        ]
      })
    ).toEqual([
      { addonId: 'owo', enabled: true, devices: null },
      { addonId: 'bhaptics', enabled: true, devices: null },
      { addonId: 'intiface', enabled: true, devices: 999 },
      { addonId: 'openshock', enabled: false, devices: null }
    ])
  })

  it('keeps one entry per id and ignores a non-list', () => {
    expect(parseAddonStats({ addons: [{ id: 'mcp', enabled: false }, { id: 'mcp', enabled: true }] })).toEqual([{ addonId: 'mcp', enabled: true, devices: null }])
    expect(parseAddonStats({ addons: 'nope' })).toEqual([])
    expect(parseAddonStats(null)).toEqual([])
  })
})
