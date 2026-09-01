import type { ComputedRef, Ref } from 'vue'
import { computed, ref } from 'vue'

const OPEN_SHOCK_URL = 'https://api.openshock.app'
const TOKEN_STORAGE_KEY = 'openShock_token'

export interface ShockCommand {
  id: string
  type: 'Stop' | 'Shock' | 'Vibrate' | 'Sound'
  duration: number
  intensity: number
  exclusive?: boolean
}

export interface OpenShockShocker {
  id: string
  rfId: number
  model: string
  name: string
  isPaused: boolean
  createdOn: string
}

// GET /1/shockers/own returns one entry per hub, each grouping the shockers paired to it - not
// one entry per shocker (confirmed against a live response; previously named `OpenShockDevice`,
// which read as if it were a single shocker).
export interface OpenShockHub {
  id: string
  name: string
  createdOn: string
  shockers: OpenShockShocker[]
}

// 'unconfigured': no token saved yet (or it was just cleared).
// 'checking': a validation request against the OpenShock API is in flight.
// 'valid'/'invalid': the last validation request succeeded/failed.
// Fail-closed: any state other than 'valid' means OpenShock controls are treated as unavailable,
// so a token that has never been checked this session (e.g. right after app start) doesn't let
// controls through until it's actually been verified.
export type OpenShockStatus = 'unconfigured' | 'checking' | 'valid' | 'invalid'

// Module-scope singletons - every useOpenShock() caller shares the same token/status so a change
// made in Settings is instantly reflected everywhere (ControlModal's type list, useControls'
// `unavailable` flag, the host's command enforcement), same pattern as useAuth()'s `token`.
const token = ref<string>(localStorage.getItem(TOKEN_STORAGE_KEY) ?? '')
const status = ref<OpenShockStatus>('unconfigured')

// Guards against a stale, slower request resolving after a newer one and clobbering the status.
let checkId = 0

export function useOpenShock(): {
  command: (cmd: ShockCommand[]) => Promise<void>
  getShockers: () => Promise<OpenShockHub[]>
  setToken: (token: string) => void
  getToken: () => string
  token: Ref<string>
  status: Ref<OpenShockStatus>
  isAvailable: ComputedRef<boolean>
  testConnection: () => Promise<boolean>
} {
  const makeRequest = async <T = void>(
    endpoint: string,
    options: RequestInit = { method: 'GET' }
  ): Promise<{ data: T; message: string }> => {
    const authToken = token.value

    if (!authToken) {
      throw new Error('No OpenShock token available. Please set a token first.')
    }

    try {
      const response = await fetch(`${OPEN_SHOCK_URL}${endpoint}`, {
        ...options,
        headers: {
          'Content-Type': 'application/json',
          OpenShockToken: authToken,
          ...options.headers
        }
      })

      if (!response.ok) {
        throw new Error(`OpenShock request failed with status ${response.status}`)
      }

      const contentType = response.headers.get('content-type')
      if (contentType && contentType.includes('application/json')) {
        return await response.json()
      }
      return { data: undefined as unknown as T, message: 'No content' }
    } catch (error) {
      console.error('Failed to make OpenShock request:', error)
      throw error
    }
  }

  const command = async (cmd: ShockCommand[]): Promise<void> => {
    await makeRequest('/1/shockers/control', {
      method: 'POST',
      body: JSON.stringify(cmd)
    })
  }

  const getShockers = async (): Promise<OpenShockHub[]> => {
    return (await makeRequest<OpenShockHub[]>('/1/shockers/own')).data
  }

  // Validates the currently-saved token against the OpenShock API and updates `status`
  // accordingly. Transparent by design: callers don't need to interpret the result themselves -
  // they just read `isAvailable`/`status` reactively - so this is meant to be fired automatically
  // (on save, on load) rather than from an explicit "Test connection" button.
  const testConnection = async (): Promise<boolean> => {
    if (!token.value) {
      status.value = 'unconfigured'
      return false
    }

    const id = ++checkId
    status.value = 'checking'

    try {
      await getShockers()
      if (id === checkId) status.value = 'valid'
      return true
    } catch {
      if (id === checkId) status.value = 'invalid'
      return false
    }
  }

  // No hardcoded fallback token here (there used to be one, embedded directly in source — treat
  // it as compromised and revoke it in the OpenShock dashboard if it hasn't been already). Callers
  // must configure their own token via setToken(); makeRequest() already surfaces a clear error
  // when none is set.
  const getToken = (): string => token.value

  const setToken = (value: string): void => {
    token.value = value

    if (value) localStorage.setItem(TOKEN_STORAGE_KEY, value)
    else localStorage.removeItem(TOKEN_STORAGE_KEY)

    void testConnection()
  }

  const isAvailable = computed(() => status.value === 'valid')

  return {
    command,
    getShockers,
    setToken,
    getToken,
    token,
    status,
    isAvailable,
    testConnection
  }
}

// Verify whatever token was persisted from a previous session as soon as this module loads, so
// `isAvailable` reflects reality before the user ever opens Settings or the control modal.
if (token.value) void useOpenShock().testConnection()
