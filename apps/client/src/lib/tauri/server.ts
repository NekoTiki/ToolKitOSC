// The share server: the host link, the rules viewer commands are checked against, signing in and
// the HTTP API - src-tauri/src/server/.
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import type { CommandValue } from '@toolkitosc/shared-ui'

// The share server link - mirrors `Status` in src-tauri/src/server/mod.rs. 'rejected': the server
// refused the token (auth-error). 'mismatch': it speaks another protocol version. Neither retries.
export type ServerStatus = 'closed' | 'connecting' | 'open' | 'rejected' | 'mismatch'

export interface ServerResponse {
  status: number
  // Parsed JSON, or null.
  body: unknown
}

// Who's signed in to which server - mirrors src-tauri/src/server/auth.rs. The tokens stay in Rust.
export interface AuthState {
  // False until Rust has read the tokens at startup.
  loaded: boolean
  // Server URL -> the user its token names, for every server with a token. null when it can't be
  // read.
  users: Record<string, AuthUser | null>
  secretStore: 'keyring' | 'file'
}

export interface AuthUser {
  discord: {
    id: string
    avatar: string
    name: string
  }
  username: string
  email: string
}

// The share settings Rust checks viewer commands against - mirrors src-tauri/src/server/policy.rs.
export interface ServerPolicy {
  clientType: string
  showViewers: boolean
}

export interface CommandExecuted {
  peerId: string
  groupId: string
  controlId: string
  controlName: string
  type: string
  value?: CommandValue
  // A viewer's own pattern: its samples, and the run that plays it (onPatternEnded carries it).
  samples?: number[]
  patternRun?: number | null
}

export const serverApi = {
  // The rules viewer commands are checked against, in Rust.
  serverSetPolicy: (policy: ServerPolicy): Promise<void> => invoke('server_set_policy', { policy }),
  // A viewer's command Rust checked and ran (or accepted without a control to run), to log.
  onCommandExecuted: (callback: (executed: CommandExecuted) => void): void => {
    void listen<CommandExecuted>('command-executed', (event) => callback(event.payload))
  },
  // Signs in with the token Rust keeps for that server. Asking again for the link that's already up
  // keeps it and replays its status, auth-success and client-list, which is how a reloaded webview
  // catches up.
  serverConnect: (url: string, protocolVersion: number): Promise<void> =>
    invoke('server_connect', { url, protocolVersion }),
  // A request to the server's HTTP API, signed with that server's token. Rejects when signed out
  // or unreachable; an error status comes back as is.
  serverRequest: (url: string, method: 'GET' | 'POST' | 'PUT' | 'DELETE', path: string, body?: unknown): Promise<ServerResponse> =>
    invoke('server_request', { url, method, path, body: body ?? null }),
  authState: (): Promise<AuthState> => invoke('auth_state'),
  // Once, from the tokens the webview used to keep in localStorage (server URL -> JWT).
  authMigrate: (tokens: Record<string, string>): Promise<void> => invoke('auth_migrate', { tokens }),
  authSignOut: (url: string): Promise<void> => invoke('auth_sign_out', { url }),
  // Starts signing in to that server, if it isn't already. The browser link arrives on onAuthUri.
  authSignIn: (url: string): Promise<void> => invoke('auth_sign_in', { url }),
  authSignInCancel: (): Promise<void> => invoke('auth_sign_in_cancel'),
  onAuthChanged: (callback: (state: AuthState) => void): void => {
    void listen<AuthState>('auth-changed', (event) => callback(event.payload))
  },
  onAuthUri: (callback: (uri: string | null) => void): void => {
    void listen<string | null>('auth-uri', (event) => callback(event.payload))
  },
  serverDisconnect: (): Promise<void> => invoke('server_disconnect'),
  // One `{type, message}` envelope as JSON text. Dropped while the link is down.
  serverSend: (text: string): Promise<void> => invoke('server_send', { text }),
  // These three resolve once the listener is registered.
  onServerStatus: (callback: (status: ServerStatus) => void): Promise<unknown> =>
    listen<ServerStatus>('server-status', (event) => callback(event.payload)),
  // A message from the server, as JSON text.
  onServerMessage: (callback: (text: string) => void): Promise<unknown> =>
    listen<string>('server-message', (event) => callback(event.payload)),
  // Dev builds: a message Rust sent by itself (auth-token with the token blanked, args-update).
  onServerDebug: (callback: (text: string) => void): Promise<unknown> =>
    listen<string>('server-debug', (event) => callback(event.payload))
}
