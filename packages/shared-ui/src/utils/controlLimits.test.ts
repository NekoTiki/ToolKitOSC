import { describe, expect, it } from 'vitest'

import type { CommandWithoutIds, ControlType } from '../types/controls'
import { resolveControlForViewer, sanitizeCommand } from './controlLimits'
import vectors from './controlLimits.vectors.json'

interface Vector {
  name: string
  control: ControlType
  groupIds?: string[]
  command: CommandWithoutIds
  expected: CommandWithoutIds | null
  resolved?: { locked: boolean; limits: unknown }
}

// The host runs a Rust copy of these checks (apps/client/src-tauri/src/server/limits.rs) against
// the same cases, so the relay and the host always agree on what a viewer may do.
describe('controlLimits', () => {
  it.each(vectors as unknown as Vector[])('$name', (vector) => {
    const resolved = resolveControlForViewer(vector.control, vector.groupIds ?? [])

    if (vector.resolved) {
      expect({ locked: !!resolved.locked, limits: resolved.limits ?? null }).toEqual(vector.resolved)
    }

    expect(sanitizeCommand(resolved, vector.command)).toEqual(vector.expected)
  })
})
