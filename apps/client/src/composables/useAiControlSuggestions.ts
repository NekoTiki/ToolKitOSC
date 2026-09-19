import { useAuth } from '@renderer/composables/useAuth'
import { useAvatarDetails } from '@renderer/composables/useAvatarDetails'
import { usePresets } from '@renderer/composables/usePresets'
import { SERVER_URL } from '@renderer/constants'
import type { ControlGroup } from '@vrc-osc-toolkit/shared-ui'
import { ref } from 'vue'

export interface AiProviderInfo {
  id: string
  label: string
  configured: boolean
}

// One-shot action + result, not a module-scope singleton like useOpenShock/useAuth - there's
// nothing here that needs to be shared/reactive across every caller at once, unlike a
// connection's live status.
export function useAiControlSuggestions(): {
  loading: ReturnType<typeof ref<boolean>>
  error: ReturnType<typeof ref<string | null>>
  listProviders: () => Promise<AiProviderInfo[]>
  generate: (provider: string) => Promise<ControlGroup[]>
} {
  const { token } = useAuth()
  const { includedParameters } = usePresets()
  const { avatarDetails } = useAvatarDetails()

  const loading = ref(false)
  const error = ref<string | null>(null)

  const authHeaders = (): HeadersInit => {
    if (!token.value) throw new Error('Not logged in - log in from Settings first')

    return { Authorization: `Bearer ${token.value}`, 'Content-Type': 'application/json' }
  }

  const listProviders = async (): Promise<AiProviderInfo[]> => {
    const response = await fetch(`${SERVER_URL}/api/ai/providers`, { headers: authHeaders() })

    if (!response.ok) throw new Error(`Failed to list AI providers (${response.status})`)

    return (await response.json()) as AiProviderInfo[]
  }

  // Uses the app's existing noisy-address filter + the user's own per-avatar excluded list
  // (usePresets().includedParameters - the same set PresetModal/the coverage banner already
  // treat as "the real parameters") rather than sending every declared parameter to the server.
  const generate = async (provider: string): Promise<ControlGroup[]> => {
    loading.value = true
    error.value = null

    try {
      if (!avatarDetails.value) throw new Error('No avatar loaded')

      const parameters = includedParameters.value.map((param) => ({
        name: param.name,
        address: param.address,
        kind: param.kind
      }))

      const response = await fetch(`${SERVER_URL}/api/ai/suggest-controls`, {
        method: 'POST',
        headers: authHeaders(),
        body: JSON.stringify({
          provider,
          avatarName: avatarDetails.value.name,
          parameters
        })
      })

      if (!response.ok) {
        const body = await response.json().catch(() => ({}))
        throw new Error(body.statusMessage || `Request failed (${response.status})`)
      }

      const { groups } = (await response.json()) as { groups: ControlGroup[] }

      return groups
    } catch (err) {
      error.value = err instanceof Error ? err.message : String(err)
      throw err
    } finally {
      loading.value = false
    }
  }

  return { loading, error, listProviders, generate }
}
