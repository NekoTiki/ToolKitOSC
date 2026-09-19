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
  // Credits left today in this provider's shared pool (see the server's rateLimit.ts) - every
  // profile spends from the same number at a different rate (AiProfileInfo.cost below), so this
  // doesn't change when you switch profiles, only when you switch provider or spend some.
  remainingCredits: number
}

export interface AiProfileInfo {
  id: string
  label: string
  description: string
  // Credits one generation at this profile spends from the selected provider's pool.
  cost: number
}

export interface AiAvatarLimitInfo {
  max: number
  // null when no avatar is loaded yet - listOptions() only asks the server for this once an
  // avatarId is actually known.
  remaining: number | null
}

export interface AiOptions {
  providers: AiProviderInfo[]
  profiles: AiProfileInfo[]
  dailyCredits: number
  avatarLimit: AiAvatarLimitInfo
}

// One-shot action + result, not a module-scope singleton like useOpenShock/useAuth - there's
// nothing here that needs to be shared/reactive across every caller at once, unlike a
// connection's live status.
export function useAiControlSuggestions(): {
  loading: ReturnType<typeof ref<boolean>>
  error: ReturnType<typeof ref<string | null>>
  listOptions: () => Promise<AiOptions>
  generate: (provider: string, profile: string) => Promise<ControlGroup[]>
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

  // Model choice is never part of this - the server picks it from (provider, profile) alone (see
  // profiles.ts's resolveModel). This only ever offers the two axes the server actually exposes.
  // Always re-fetched (never cached here) - credits/avatar-remaining change every time generate()
  // runs, and the modal that owns this composable's lifetime doesn't get remounted on every open
  // (see AiControlsModal.vue), so a caller has to explicitly call this again to see fresh numbers.
  const listOptions = async (): Promise<AiOptions> => {
    const avatarId = avatarDetails.value?.id
    const query = avatarId ? `?avatarId=${encodeURIComponent(avatarId)}` : ''
    const response = await fetch(`${SERVER_URL}/api/ai/providers${query}`, { headers: authHeaders() })

    if (!response.ok) throw new Error(`Failed to list AI options (${response.status})`)

    return (await response.json()) as AiOptions
  }

  // Uses the app's existing noisy-address filter + the user's own per-avatar excluded list
  // (usePresets().includedParameters - the same set PresetModal/the coverage banner already
  // treat as "the real parameters") rather than sending every declared parameter to the server.
  const generate = async (provider: string, profile: string): Promise<ControlGroup[]> => {
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
          profile,
          avatarId: avatarDetails.value.id,
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

  return { loading, error, listOptions, generate }
}
