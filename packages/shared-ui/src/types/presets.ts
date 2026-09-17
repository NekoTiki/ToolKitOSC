// Preset domain model — a named, avatar-scoped snapshot of OSC parameter values, persisted by the
// desktop client as one JSON file per avatar (see apps/client/src-tauri/src/presets.rs), mirrored
// here so both the authoring UI and a 'preset' ControlType (see controls.ts) can share one shape.
import type { OSCArg } from './osc'

export interface PresetParameter {
  address: string
  name: string
  kind: 'Bool' | 'Float' | 'Int'
  value: OSCArg
}

export interface Preset {
  id: string
  avatarId: string
  avatarName: string
  name: string
  icon?: string
  createdAt: string
  updatedAt: string
  parameters: PresetParameter[]
}

// The full on-disk shape of one avatar's `presets-<avatarId>.json` - `excludedAddresses` is a
// per-avatar list of OSC input addresses the user has chosen to hide from preset capture/editing,
// on top of the hardcoded noisy-address denylist (see apps/client's addressFilter.ts) that's
// already applied everywhere addresses are picked.
export interface PresetStore {
  excludedAddresses: string[]
  presets: Preset[]
}
