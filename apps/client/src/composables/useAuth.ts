import { serverWsUrl } from '@renderer/composables/useWebsocketSettings'
import type { AuthState, AuthUser } from '@renderer/lib/tauri-bridge'
import { api } from '@renderer/lib/tauri-bridge'
import type { ComputedRef } from 'vue'
import { computed, ref, watch } from 'vue'

// Where the webview kept the tokens before Rust took them over (see src-tauri/src/server/auth.rs).
// Handed over once, then removed.
const LEGACY_TOKENS_KEY = 'authTokens'
const LEGACY_TOKEN_KEY = 'authToken'
const KNOWN_SERVERS_KEY = 'knownServers'

// Rust keeps one token per server URL, so switching servers (see useWebsocketSettings) doesn't
// require logging in again on a server that's already been authenticated against. Only the user
// each token names comes back here, never the token.
const state = ref<AuthState | null>(null)

api.onAuthChanged((value) => (state.value = value))
void api.authState().then((value) => (state.value ??= value))

// Every server the user has ever signed in to, kept apart from the tokens so signing out (or the
// server rejecting a stale token) doesn't drop it from the server picker. Read synchronously like
// useWebsocketSettings: the client has no SSR.
const knownServers = ref<string[]>(loadKnownServers())

saveKnownServers(knownServers.value)
migrateLegacyTokens()

function loadLegacyTokens(): Record<string, string> {
  try {
    const stored: unknown = JSON.parse(localStorage.getItem(LEGACY_TOKENS_KEY) || '{}')

    return stored && typeof stored === 'object' ? (stored as Record<string, string>) : {}
  } catch {
    return {}
  }
}

function loadKnownServers(): string[] {
  try {
    const stored: unknown = JSON.parse(localStorage.getItem(KNOWN_SERVERS_KEY) || '[]')
    const list = Array.isArray(stored) ? stored.filter((url): url is string => typeof url === 'string') : []

    // Servers signed in to before this list existed only show up as token keys.
    return [...new Set([...list, ...Object.keys(loadLegacyTokens())])]
  } catch {
    return Object.keys(loadLegacyTokens())
  }
}

function saveKnownServers(list: string[]): void {
  knownServers.value = list
  localStorage.setItem(KNOWN_SERVERS_KEY, JSON.stringify(list))
}

function migrateLegacyTokens(): void {
  const tokens = loadLegacyTokens()
  // The pre-per-server token wasn't tracked against a server URL, so the best we can do is treat
  // it as the token for whichever server is current right now.
  const legacy = localStorage.getItem(LEGACY_TOKEN_KEY)

  if (legacy && !tokens[serverWsUrl.value]) tokens[serverWsUrl.value] = legacy
  if (!Object.keys(tokens).length && localStorage.getItem(LEGACY_TOKENS_KEY) === null) return

  void api
    .authMigrate(tokens)
    .then(() => {
      localStorage.removeItem(LEGACY_TOKENS_KEY)
      localStorage.removeItem(LEGACY_TOKEN_KEY)
    })
    .catch((error) => console.error('Could not move the sign-in tokens to the app:', error))
}

// A token only ever arrives after a successful sign-in, so that's when a server joins the known
// list. Signing out leaves the server listed.
watch(
  () => Object.keys(state.value?.users ?? {}),
  (urls) => {
    const missing = urls.filter((url) => !knownServers.value.includes(url))

    if (missing.length) saveKnownServers([...knownServers.value, ...missing])
  }
)

// Until then nobody counts as signed out either.
const ready = computed(() => !!state.value?.loaded)
const loggedIn = computed(() => !!state.value && serverWsUrl.value in state.value.users)
const user = computed<AuthUser | null>(() => state.value?.users[serverWsUrl.value] ?? null)

export function useAuth(): {
  ready: ComputedRef<boolean>
  loggedIn: ComputedRef<boolean>
  user: ComputedRef<AuthUser | null>
  knownServers: ComputedRef<string[]>
  secretStore: ComputedRef<AuthState['secretStore'] | undefined>
  hasToken: (url: string) => boolean
  signOut: () => Promise<void>
  forgetServer: (url: string) => void
} {
  const hasToken = (url: string): boolean => !!state.value && url in state.value.users

  // Forgets this server's token only; tokens for other servers stay.
  const signOut = (): Promise<void> => api.authSignOut(serverWsUrl.value)

  // Removes a server from the picker, along with any token still stored for it.
  const forgetServer = (url: string): void => {
    saveKnownServers(knownServers.value.filter((known) => known !== url))

    if (hasToken(url)) void api.authSignOut(url)
  }

  return {
    ready,
    loggedIn,
    user,
    knownServers: computed(() => knownServers.value),
    secretStore: computed(() => state.value?.secretStore),
    hasToken,
    signOut,
    forgetServer
  }
}
