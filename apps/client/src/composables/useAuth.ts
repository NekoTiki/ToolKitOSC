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

// Tokens keyed by server URL, so switching servers (see useWebsocketSettings) doesn't require
// logging in again on a server that's already been authenticated against.
const tokens = ref<Record<string, string>>({})

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
  setToken: (token?: string) => void
} {
  const token = computed(() => tokens.value[serverWsUrl.value])
  const loggedIn = computed(() => !!token.value)

  // Servers the user has successfully logged into before - a token only ever lands here via
  // setToken(), which only runs on a successful auth - so this doubles as "known good servers"
  // for suggesting in the server URL picker.
  const knownServers = computed(() => Object.keys(tokens.value))

  const setToken = (newToken?: string): void => {
    const next = { ...tokens.value }

    if (newToken) next[serverWsUrl.value] = newToken
    else delete next[serverWsUrl.value]

    tokens.value = next
    localStorage.setItem(STORAGE_KEY, JSON.stringify(next))
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

  return { loggedIn, token, user, knownServers, setToken }
}
