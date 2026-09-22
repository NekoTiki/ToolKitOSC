// Typed WS message envelope, replacing the stringly-typed `{type, message}` blobs previously
// hand-parsed independently in useWebsocketHost.ts (client), useWebsocketClient.ts (server app),
// and the Nitro routes server/routes/host.ts + server/routes/ws/[ws].ts. Shapes below are taken
// directly from those call sites, not guessed.
import type { ControlCommand, ControlGroup, ControlTypes } from './controls'
import type { OpenShockControlValue } from './openShock'
import type { OSCArg } from './osc'
import type { ColorFamilyWithoutNeutral } from './theme'

export interface WsEnvelope<Type extends string, Message> {
  type: Type
  message: Message
}

// Bumped only when a wire-format change actually breaks compatibility (a message shape changes or
// a required field is added/removed) — not on every feature release. The desktop client reports
// this at auth-token time so the server (the one long-lived instance, redeployed independently of
// any given client build - see apps/server/deploy.mjs) can tell a stale client apart from a bad
// token, instead of silently misparsing or dropping its messages.
//
// v2: 'controls-update' (host -> server) changed from a bare ControlGroup[] to
// ControlsUpdateMessage (adds `avatarId`) - see that type's own comment.
export const PROTOCOL_VERSION = 2

// Oldest client PROTOCOL_VERSION the server still accepts. Raise this only once no compatibility
// shim is needed for versions below it; for now (no shim exists yet) it tracks PROTOCOL_VERSION.
export const MIN_SUPPORTED_PROTOCOL_VERSION = 2

export interface ProtocolMismatchMessage {
  reason: string
  serverVersion: number
  minSupportedVersion: number
}

export interface AuthSuccessMessage {
  message: string
  serverVersion: number
  // Every control type this server's bundled shared-ui build currently knows how to render (see
  // KNOWN_CONTROL_TYPES in controls.ts) - lets the client warn when a control it just created uses
  // a type this server (and so the browser viewer page, which is served from the same build)
  // can't display yet, without needing a full protocol-version bump/disconnect over it: the server
  // itself doesn't care what a control's `type` is, it only stores and relays the JSON.
  supportedControlTypes: ControlTypes[]
  // This account's current AI feature access state (see apps/server's utils/ai/access.ts), so the
  // client has it the instant it connects instead of needing a separate round trip - kept live
  // afterward by 'ai-access-update' pushes on this same connection.
  aiAccess: AiAccessUpdateMessage
}

// Pushed whenever an admin grants/revokes AI access or changes the model-select permission for
// this account from /dashboard (see apps/server's api/admin/users/*) - lets the desktop client
// dynamically show/hide the "AI Generate" button and provider/profile pickers without needing to
// reconnect or re-poll for it.
export interface AiAccessUpdateMessage {
  hasAccess: boolean
  canSelectModel: boolean
}

// Pushed right after a POST /api/ai/suggest-controls request spends from the account's credit pool
// (see apps/server's rateLimit.ts) - sent over the same host connection rather than left for the
// client to notice by polling GET /api/ai/providers again, so the AI modal's displayed balance
// updates the moment a generation is actually charged for, whether or not it goes on to succeed.
export interface AiCreditsUpdateMessage {
  // One overall pool per account per day, shared across every provider (see rateLimit.ts) - not
  // per-provider.
  remainingCredits: number
  dailyCredits: number
}

// POST /api/ai/suggest-controls now only kicks a generation off and returns { requestId } right
// away - a Cloudflare tunnel in front of the server enforces a ~2.1 minute request timeout,
// comfortably shorter than a Heavy-profile generation can take, so the actual result is delivered
// asynchronously over this same host connection instead, correlated by that requestId. Progress
// messages are purely cosmetic flavor text (see apps/server's suggest-controls.post.ts) so the
// client's "Generating…" state doesn't look frozen for however long the upstream provider takes.
export interface AiGenerateProgressMessage {
  requestId: string
  message: string
}

export interface AiGenerateResultMessage {
  requestId: string
  groups: ControlGroup[]
}

export interface AiGenerateErrorMessage {
  requestId: string
  message: string
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

// One shape, sent from either of two chokepoints - the relay itself, when a viewer's raw message
// rate trips its own ingress limit (apps/server's ws/[ws].ts, straight to that same peer, no host
// round trip needed), or the host, when a specific client's *executed* commands trip its own
// per-client limit (apps/client's useWebsocketHost.ts, relayed on to that client the same way
// client-invalid/client-banned already are - see host.ts). `peerId` is only meaningful for the
// second case (routing through sendToClient); the relay's own direct send already knows who to
// reach, but fills it in too so ServerToViewerMessage's handler doesn't need to care which
// chokepoint a given message came from.
export interface RateLimitedMessage {
  peerId: string
  reason: string
}

// Mirrors BanScope in apps/client/src/db/clients.db.ts (duplicated rather than imported, same as
// ClientType above - the desktop app's local db types aren't shared through this package).
export type BanScope = 'ip' | 'discord'

export interface ClientBannedMessage {
  peerId: string
  scope: BanScope
  reason?: string
}

// Controls are authored/stored per-avatar client-side (see apps/client's useControls.ts, keyed by
// `controls_${avatarId}` in localStorage) - a bare ControlGroup[] here would only ever tell the
// server about whichever avatar happens to be loaded right now, with no way to distinguish that
// from "this account's controls overall" (see apps/server's controlStats.ts, which keys its
// inventory snapshot by (discordId, avatarId) precisely so switching avatars doesn't wipe out the
// previous one's counts). `avatarId` is null only in the brief window before OSCQuery has reported
// any avatar at all (see useAvatarDetails.ts) - the server skips recording inventory for that case,
// since there's nothing to key it by yet.
export interface ControlsUpdateMessage {
  avatarId: string | null
  groups: ControlGroup[]
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
  | WsEnvelope<'auth-token', { token: string; protocolVersion: number }>
  | WsEnvelope<'controls-update', ControlsUpdateMessage>
  | WsEnvelope<'args-initial', Record<string, OSCArg[]>>
  | WsEnvelope<'args-update', ArgUpdateMessage>
  | WsEnvelope<'theme-update', ThemeUpdateMessage>
  | WsEnvelope<'open-shock-value-update', OpenShockValueUpdateMessage>
  | WsEnvelope<'intiface-value-update', IntifaceValueUpdateMessage>
  | WsEnvelope<'intiface-pattern-value-update', IntifacePatternValueUpdateMessage>
  | WsEnvelope<'client-invalid', ClientInvalidMessage>
  | WsEnvelope<'client-banned', ClientBannedMessage>
  | WsEnvelope<'rate-limited', RateLimitedMessage>

// Server -> desktop client, over /host. `command`/`update-username` are relays of a viewer's
// message with a `from` (session id) field appended directly onto the envelope, not nested.
export type ServerToHostMessage =
  | WsEnvelope<'auth-success', AuthSuccessMessage>
  | WsEnvelope<'auth-error', string>
  | WsEnvelope<'protocol-mismatch', ProtocolMismatchMessage>
  | (WsEnvelope<'command', ControlCommand> & { from: string })
  | (WsEnvelope<'update-username', { displayName: string }> & { from: string })
  | WsEnvelope<'client-list', ClientListEntry[]>
  | WsEnvelope<'ai-credits-update', AiCreditsUpdateMessage>
  | WsEnvelope<'ai-access-update', AiAccessUpdateMessage>
  | WsEnvelope<'ai-generate-progress', AiGenerateProgressMessage>
  | WsEnvelope<'ai-generate-result', AiGenerateResultMessage>
  | WsEnvelope<'ai-generate-error', AiGenerateErrorMessage>

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
  | WsEnvelope<'rate-limited', RateLimitedMessage>
  | WsEnvelope<'theme-update', ThemeUpdateMessage>
  | WsEnvelope<'welcome', string>

// Browser viewer -> server, over /ws/[roomId].
export type ViewerToServerMessage =
  | WsEnvelope<'command', ControlCommand>
  | WsEnvelope<'update-username', { displayName: string }>
