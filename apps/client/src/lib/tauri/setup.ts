// Your controls and lock profiles, kept per avatar by Rust - src-tauri/src/setup/.
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import type { ControlGroup, ControlLimits, CreatedBy, ViewerGroupInfo, ViewerGroupRule } from '@toolkitosc/shared-ui'

export type GroupOverrides = Record<string, ViewerGroupRule>

export type LockedControlGroup = {
  id: string
  name: string
  lockedControls: Record<string, boolean>
  // Per-control limits for viewers, keyed by control id.
  limits?: Record<string, ControlLimits>
  // What each viewer group gets instead, by group id, then control id. A control a group has no
  // rule for is the same for the group as for everyone.
  groupOverrides?: Record<string, GroupOverrides>
  // Set while it's an AI assistant's draft. Saving it yourself makes it yours.
  createdBy?: CreatedBy
}

// The worn avatar's controls, each with the active profile's locks, limits and group rules and
// whether it's unavailable, and its profiles.
export interface SetupSnapshot {
  avatarId: string | null
  groups: ControlGroup[]
  profiles: LockedControlGroup[]
  defaultProfile: string | null
  activeProfile: string | null
}

// One avatar's setup as localStorage kept it, for the hand-over to Rust.
export interface LegacySetup {
  avatarId: string
  groups: ControlGroup[]
  profiles: LockedControlGroup[]
  defaultProfile: string | null
}

export interface GroupMembership {
  id: string
  // When a temporary membership ends, in ms. Unset: until the host removes them.
  until?: number
}

export type BanScope = 'ip' | 'discord'

// A ban targets either a stable IP or a Discord ID, never a pair of the two.
export interface Ban {
  scope: BanScope
  value: string
  reason?: string
  createdAt: number
}

// Viewer groups you made, who is in which (by viewer key, expired ones included) and bans.
export interface ViewersSetup {
  groups: ViewerGroupInfo[]
  memberships: Record<string, GroupMembership[]>
  bans: Ban[]
  // Discord ids let in, for "Only people you let in".
  approved: string[]
}

export const setupApi = {
  setupControls: (): Promise<SetupSnapshot> => invoke('setup_controls'),
  // Saves the worn avatar's groups and their controls, in this order.
  setupSaveGroups: (avatarId: string, groups: ControlGroup[]): Promise<void> => invoke('setup_save_groups', { avatarId, groups }),
  setupSaveProfiles: (avatarId: string, profiles: LockedControlGroup[], defaultProfile: string | null): Promise<void> =>
    invoke('setup_save_profiles', { avatarId, profiles, defaultProfile }),
  // Turns a profile on, or none, until the app closes.
  setupSwitchProfile: (profileId: string | null): Promise<void> => invoke('setup_switch_profile', { profileId }),
  // The type of every control saved for any avatar.
  setupSavedControlTypes: (): Promise<string[]> => invoke('setup_saved_control_types'),
  setupMigrate: (avatars: LegacySetup[]): Promise<void> => invoke('setup_migrate', { avatars }),
  onSetupControls: (callback: (snapshot: SetupSnapshot) => void): void => {
    void listen<SetupSnapshot>('setup-controls', (event) => callback(event.payload))
  },
  setupViewers: (): Promise<ViewersSetup> => invoke('setup_viewers'),
  // The viewer groups you made, in this order. Anyone in a group that's gone is taken out.
  setupSaveViewerGroups: (groups: ViewerGroupInfo[]): Promise<void> => invoke('setup_save_viewer_groups', { groups }),
  setupAddToGroup: (key: string, groupId: string, durationMs: number | null): Promise<void> =>
    invoke('setup_add_to_group', { key, groupId, durationMs }),
  setupRemoveFromGroup: (key: string, groupId: string): Promise<void> => invoke('setup_remove_from_group', { key, groupId }),
  setupBan: (scope: BanScope, value: string, reason?: string): Promise<void> => invoke('setup_ban', { scope, value, reason: reason ?? null }),
  setupUnban: (scope: BanScope, value: string): Promise<void> => invoke('setup_unban', { scope, value }),
  setupSetApproved: (discordId: string, approved: boolean): Promise<void> => invoke('setup_set_approved', { discordId, approved }),
  setupMigrateViewers: (viewers: ViewersSetup): Promise<void> => invoke('setup_migrate_viewers', { viewers }),
  onSetupViewers: (callback: (viewers: ViewersSetup) => void): void => {
    void listen<ViewersSetup>('setup-viewers', (event) => callback(event.payload))
  }
}
