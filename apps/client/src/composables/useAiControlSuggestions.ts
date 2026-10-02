import { useAiGenerationStatus } from '@renderer/composables/useAiGenerationStatus'
import { useAuth } from '@renderer/composables/useAuth'
import { useAvatarDetails } from '@renderer/composables/useAvatarDetails'
import { usePresets } from '@renderer/composables/usePresets'
import { serverHttpUrl } from '@renderer/composables/useWebsocketSettings'
import type { ControlGroup } from '@toolkitosc/shared-ui'
import { ref } from 'vue'

export interface AiProviderInfo {
  id: string
  label: string
  configured: boolean
}

export interface AiProfileInfo {
  id: string
  label: string
  description: string
  // Credits one generation at this profile spends from the account's overall daily pool.
  cost: number
}

export interface AiOptions {
  providers: AiProviderInfo[]
  profiles: AiProfileInfo[]
  // One overall pool per account per day (see the server's rateLimit.ts), shared across every
  // provider - not per-provider, so this doesn't change when you switch provider, only profile or
  // actually spending some.
  remainingCredits: number
  dailyCredits: number
  // Whether this account is allowed to pick the provider/model itself (see the server's
  // utils/ai/access.ts) - profile is always the user's own choice regardless. false means
  // the AI page hides the provider picker; the server still picks one on its own,
  // it's just not surfaced to the user (see resolveDefaultProvider's load-balancing comment -
  // there's no single fixed answer to show anyway).
  canSelectModel: boolean
}

// One-shot action + result, not a module-scope singleton like useOpenShock/useAuth - there's
// nothing here that needs to be shared/reactive across every caller at once, unlike a
// connection's live status.
export function useAiControlSuggestions(): {
  loading: ReturnType<typeof ref<boolean>>
  error: ReturnType<typeof ref<string | null>>
  listOptions: () => Promise<AiOptions>
  generate: (provider?: string, profile?: string) => Promise<ControlGroup[]>
} {
  const { token } = useAuth()
  const { includedParameters } = usePresets()
  const { avatarDetails } = useAvatarDetails()
  const { waitForResult } = useAiGenerationStatus()

  const loading = ref(false)
  const error = ref<string | null>(null)

  const authHeaders = (): HeadersInit => {
    if (!token.value) throw new Error('Not logged in - log in from Settings first')

    return { Authorization: `Bearer ${token.value}`, 'Content-Type': 'application/json' }
  }

  // Model choice is never part of this - the server picks it from (provider, profile) alone (see
  // profiles.ts's resolveModel). This only ever offers the two axes the server actually exposes.
  // Always re-fetched (never cached here) - remaining credits change every time generate() runs,
  // and the modal that owns this composable's lifetime doesn't get remounted on every open (see
  // the AI page), so a caller has to explicitly call this again to see a fresh number.
  const listOptions = async (): Promise<AiOptions> => {
    const response = await fetch(`${serverHttpUrl.value}/api/ai/providers`, { headers: authHeaders() })

    if (!response.ok) throw new Error(`Failed to list AI options (${response.status})`)

    return (await response.json()) as AiOptions
  }

  // Uses the app's existing noisy-address filter + the user's own per-avatar excluded list
  // (usePresets().includedParameters - the same set PresetModal/the coverage banner already
  // treat as "the real parameters") rather than sending every declared parameter to the server.
  //
  // This POST only ever kicks the generation off and gets back a requestId - the actual result
  // arrives asynchronously over the host WS connection (see useAiGenerationStatus.ts), since a
  // Cloudflare tunnel in front of the server enforces a ~2.1 minute request timeout that a real
  // generation (especially Heavy) can easily exceed. waitForResult resolves/rejects once that
  // matching 'ai-generate-result'/'ai-generate-error' push arrives.
  const generate = async (provider?: string, profile?: string): Promise<ControlGroup[]> => {
    loading.value = true
    error.value = null

    try {
      if (!avatarDetails.value) throw new Error('No avatar loaded')

      const parameters = includedParameters.value.map((param) => ({
        name: param.name,
        address: param.address,
        kind: param.kind
      }))

      const response = await fetch(`${serverHttpUrl.value}/api/ai/suggest-controls`, {
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

      const { requestId } = (await response.json()) as { requestId: string }

      return await waitForResult(requestId)
    } catch (err) {
      error.value = err instanceof Error ? err.message : String(err)
      throw err
    } finally {
      loading.value = false
    }
  }

  return { loading, error, listOptions, generate }
}
