// Typed WS message envelope, replacing the stringly-typed `{type, message}` blobs previously
// hand-parsed independently in useWebsocketHost.ts (client), useWebsocketClient.ts (server app),
// and the Nitro routes server/routes/host.ts + server/routes/ws/[ws].ts. Shapes below are taken
// directly from those call sites, not guessed.
import type { ControlCommand, ControlGroup } from './controls'
import type { OpenShockControlValue } from './openShock'
import type { OSCArg } from './osc'
import type { ColorFamilyWithoutNeutral } from './theme'

export interface WsEnvelope<Type extends string, Message> {
  type: Type
  message: Message
}

export type ClientType = 'everyone' | 'username' | 'discord'

export interface ArgUpdateMessage {
  address: string
  args: OSCArg[]
}

export interface ThemeUpdateMessage {
  primary: ColorFamilyWithoutNeutral | null
  secondary: ColorFamilyWithoutNeutral | null
}

export interface OpenShockValueUpdateMessage {
  controlId: string
  value: OpenShockControlValue
}

// Broadcasts the value last sent to an 'intiface-toy' control's actuators, purely so every
// viewer's slider (and the host's own) shows the same position - mirrors OpenShockValueUpdateMessage,
// but the value itself is just the raw slider value (0-1), not a struct to animate.
export interface IntifaceValueUpdateMessage {
  controlId: string
  value: number
}

// Broadcasts the pattern id (or 'off') currently playing for an 'intiface-pattern' control, purely
// so every viewer's chip picker (and the host's own) highlights the same choice - mirrors
// IntifaceValueUpdateMessage, but the value is a pattern id string instead of a raw slider value.
export interface IntifacePatternValueUpdateMessage {
  controlId: string
  value: string
}

export interface ClientInvalidMessage {
  peerId: string
  reason: string
  type: ClientType
}

// Mirrors BanScope in apps/client/src/db/clients.db.ts (duplicated rather than imported, same as
// ClientType above - the desktop app's local db types aren't shared through this package).
export type BanScope = 'ip' | 'discord'

export interface ClientBannedMessage {
  peerId: string
  scope: BanScope
  reason?: string
}

export interface ClientListEntry {
  peerId: string
  ip: string
  user: {
    discord?: { id: string; avatar: string; name: string }
    userDefinedDisplayName?: string
    username?: string
    email?: string
  } | null
  sessionId?: string
}

// Desktop client ("host") -> server, over /host. `args-initial` is accepted by the Nitro route
// (server/routes/host.ts) but the current client never sends it (it only ever sends per-address
// `args-update`) — kept here to match what the server actually handles, not just what's used today.
export type HostToServerMessage =
  | WsEnvelope<'auth-token', { token: string }>
  | WsEnvelope<'controls-update', ControlGroup[]>
  | WsEnvelope<'args-initial', Record<string, OSCArg[]>>
  | WsEnvelope<'args-update', ArgUpdateMessage>
  | WsEnvelope<'theme-update', ThemeUpdateMessage>
  | WsEnvelope<'open-shock-value-update', OpenShockValueUpdateMessage>
  | WsEnvelope<'intiface-value-update', IntifaceValueUpdateMessage>
  | WsEnvelope<'intiface-pattern-value-update', IntifacePatternValueUpdateMessage>
  | WsEnvelope<'client-invalid', ClientInvalidMessage>
  | WsEnvelope<'client-banned', ClientBannedMessage>

// Server -> desktop client, over /host. `command`/`update-username` are relays of a viewer's
// message with a `from` (session id) field appended directly onto the envelope, not nested.
export type ServerToHostMessage =
  | WsEnvelope<'auth-success', string>
  | WsEnvelope<'auth-error', string>
  | (WsEnvelope<'command', ControlCommand> & { from: string })
  | (WsEnvelope<'update-username', { displayName: string }> & { from: string })
  | WsEnvelope<'client-list', ClientListEntry[]>

// Server -> browser viewer, over /ws/[roomId].
export type ServerToViewerMessage =
  | WsEnvelope<'controls-update', ControlGroup[]>
  | WsEnvelope<'args-initial', Record<string, OSCArg[]>>
  | WsEnvelope<'args-update', ArgUpdateMessage>
  | WsEnvelope<'host-status', 'online' | 'offline'>
  | WsEnvelope<'open-shock-value-update', OpenShockValueUpdateMessage>
  | WsEnvelope<'intiface-value-update', IntifaceValueUpdateMessage>
  | WsEnvelope<'intiface-pattern-value-update', IntifacePatternValueUpdateMessage>
  | WsEnvelope<'client-invalid', ClientInvalidMessage>
  | WsEnvelope<'client-banned', ClientBannedMessage>
  | WsEnvelope<'theme-update', ThemeUpdateMessage>
  | WsEnvelope<'welcome', string>

// Browser viewer -> server, over /ws/[roomId].
export type ViewerToServerMessage =
  | WsEnvelope<'command', ControlCommand>
  | WsEnvelope<'update-username', { displayName: string }>
