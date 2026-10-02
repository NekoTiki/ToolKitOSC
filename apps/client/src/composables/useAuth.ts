import { serverWsUrl } from '@renderer/composables/useWebsocketSettings'
import * as jose from 'jose'
import type { ComputedRef } from 'vue'
import { computed, onMounted, ref } from 'vue'

interface User {
  discord: {
    id: string
    avatar: string
    name: string
  }
  username: string
  email: string
}

const STORAGE_KEY = 'authTokens'
const LEGACY_STORAGE_KEY = 'authToken'
const KNOWN_SERVERS_KEY = 'knownServers'

// Tokens keyed by server URL, so switching servers (see useWebsocketSettings) doesn't require
// logging in again on a server that's already been authenticated against.
const tokens = ref<Record<string, string>>({})

// Every server the user has ever signed in to, kept apart from the tokens so signing out (or the
// server rejecting a stale token) doesn't drop it from the server picker. Read synchronously like
// useWebsocketSettings: the client has no SSR.
const knownServers = ref<string[]>(loadKnownServers())

// Saved straight away, so servers picked up from the tokens survive those tokens being cleared.
localStorage.setItem(KNOWN_SERVERS_KEY, JSON.stringify(knownServers.value))

function loadKnownServers(): string[] {
  try {
    const stored: unknown = JSON.parse(localStorage.getItem(KNOWN_SERVERS_KEY) || '[]')
    const list = Array.isArray(stored) ? stored.filter((url): url is string => typeof url === 'string') : []

    // Servers signed in to before this list existed only show up as token keys.
    return [...new Set([...list, ...Object.keys(loadTokens())])]
  } catch {
    return Object.keys(loadTokens())
  }
}

function saveKnownServers(list: string[]): void {
  knownServers.value = list
  localStorage.setItem(KNOWN_SERVERS_KEY, JSON.stringify(list))
}

function loadTokens(): Record<string, string> {
  try {
    const stored: unknown = JSON.parse(localStorage.getItem(STORAGE_KEY) || '{}')

    return stored && typeof stored === 'object' ? (stored as Record<string, string>) : {}
  } catch {
    return {}
  }
}

export function useAuth(): {
  loggedIn: ComputedRef<boolean>
  token: ComputedRef<string | undefined>
  user: ComputedRef<User | null>
  knownServers: ComputedRef<string[]>
  hasToken: (url: string) => boolean
  setToken: (token?: string) => void
  forgetServer: (url: string) => void
} {
  const token = computed(() => tokens.value[serverWsUrl.value])
  const loggedIn = computed(() => !!token.value)

  const saveTokens = (next: Record<string, string>): void => {
    tokens.value = next
    localStorage.setItem(STORAGE_KEY, JSON.stringify(next))
  }

  // A token only ever arrives here after a successful sign-in, so that's when a server joins the
  // known list. Clearing the token leaves the server listed.
  const setToken = (newToken?: string): void => {
    const next = { ...tokens.value }

    if (newToken) {
      next[serverWsUrl.value] = newToken
      if (!knownServers.value.includes(serverWsUrl.value)) saveKnownServers([...knownServers.value, serverWsUrl.value])
    } else delete next[serverWsUrl.value]

    saveTokens(next)
  }

  const hasToken = (url: string): boolean => !!tokens.value[url]

  // Removes a server from the picker, along with any token still stored for it.
  const forgetServer = (url: string): void => {
    saveKnownServers(knownServers.value.filter((known) => known !== url))

    if (tokens.value[url]) {
      const next = { ...tokens.value }

      delete next[url]
      saveTokens(next)
    }
  }

  const user = computed(() => {
    if (!token.value) return null

    try {
      const decoded = jose.decodeJwt<{ user: User }>(token.value)

      return decoded.user || null
    } catch {
      return null
    }
  })

  onMounted(() => {
    tokens.value = loadTokens()

    // One-time migration from the pre-per-server token: it wasn't tracked against a server URL,
    // so the best we can do is treat it as the token for whichever server is current right now.
    const legacy = localStorage.getItem(LEGACY_STORAGE_KEY)

    if (legacy) {
      if (!tokens.value[serverWsUrl.value]) setToken(legacy)
      localStorage.removeItem(LEGACY_STORAGE_KEY)
    }
  })

  return { loggedIn, token, user, knownServers: computed(() => knownServers.value), hasToken, setToken, forgetServer }
}
