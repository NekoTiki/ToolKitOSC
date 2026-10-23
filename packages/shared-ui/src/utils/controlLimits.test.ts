import { describe, expect, it } from 'vitest'

import type { ControlType } from '../types/controls'
import { resolveControlForViewer } from './controlLimits'
import vectors from './controlLimits.vectors.json'

interface Vector {
  name: string
  control: ControlType
  groupIds?: string[]
  resolved?: { locked: boolean; limits: unknown }
}

// What a viewer is shown, against the same cases the desktop app's Rust copy runs
// (apps/client/src-tauri/src/server/limits.rs). Whether a command is let through is decided there
// alone, so the `command`/`expected` part of each case is only checked in Rust.
describe('resolveControlForViewer', () => {
  it.each((vectors as unknown as Vector[]).filter((vector) => vector.resolved))('$name', (vector) => {
    const resolved = resolveControlForViewer(vector.control, vector.groupIds ?? [])

    expect({ locked: !!resolved.locked, limits: resolved.limits ?? null }).toEqual(vector.resolved)
  })
})
