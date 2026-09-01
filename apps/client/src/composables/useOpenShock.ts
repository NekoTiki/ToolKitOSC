const OPEN_SHOCK_URL = 'https://api.openshock.app'
const TOKEN_STORAGE_KEY = 'openShock_token'

export interface ShockCommand {
  id: string
  type: 'Stop' | 'Shock' | 'Vibrate' | 'Sound'
  duration: number
  intensity: number
  exclusive?: boolean
}

export interface OpenShockDevice {
  id: string
  name: string
  createdOn: string
  shockers: {
    id: string
    rfId: number
    model: string
    name: string
    isPaused: boolean
    createdOn: string
  }[]
}

export function useOpenShock(): {
  command: (cmd: ShockCommand[]) => Promise<void>
  getShockers: () => Promise<OpenShockDevice[]>
  setToken: (token: string) => void
  getToken: () => string | null
} {
  const makeRequest = async <T = void>(
    endpoint: string,
    options: RequestInit = { method: 'GET' }
  ): Promise<{ data: T; message: string }> => {
    const authToken = getToken()

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

  const getShockers = async (): Promise<OpenShockDevice[]> => {
    return (await makeRequest<OpenShockDevice[]>('/1/shockers/own')).data
  }

  // No hardcoded fallback token here (there used to be one, embedded directly in source — treat
  // it as compromised and revoke it in the OpenShock dashboard if it hasn't been already). Callers
  // must configure their own token via setToken(); makeRequest() already surfaces a clear error
  // when none is set.
  const getToken = (): string | null => {
    return localStorage.getItem(TOKEN_STORAGE_KEY)
  }

  const setToken = (token: string): void => {
    localStorage.setItem(TOKEN_STORAGE_KEY, token)
  }

  return {
    command,
    getShockers,
    setToken,
    getToken
  }
}
