import { useAuth } from '@renderer/composables/useAuth'
import { serverHttpUrl, serverWsUrl } from '@renderer/composables/useWebsocketSettings'
import { api } from '@renderer/lib/tauri-bridge'
import { encodeShareCode } from '@toolkitosc/shared-ui'
import { computed, ref, watch } from 'vue'

interface ShareLinks {
  main: string
  custom: string | null
  // Shown on the share page while you're offline. Missing from older servers.
  offlineMessage?: string | null
}

const { user } = useAuth()

// The server's codes for this host (see the server's db/shareLinks.ts). null until loaded, or
// when the server is too old to have them: then the link falls back to the Discord id one.
const links = ref<ShareLinks | null>(null)

const load = async (): Promise<void> => {
  links.value = null

  if (!user.value?.discord?.id) return

  try {
    const response = await api.serverRequest(serverWsUrl.value, 'GET', '/api/share-link')

    if (response.status < 400) links.value = response.body as ShareLinks
  } catch {
    // Signed out or unreachable: no link until the next sign-in.
  }
}

watch(() => [serverWsUrl.value, user.value?.discord?.id], load, { immediate: true })

const shareUrl = computed<string | null>(() => {
  const code = links.value ? (links.value.custom ?? links.value.main) : user.value?.discord?.id && encodeShareCode(user.value.discord.id)

  return code ? `${serverHttpUrl.value}/s/${code}` : null
})

// Throws the server's message (bad or taken name) so the caller can show it.
const send = async (method: 'POST' | 'PUT', path: string, body?: unknown): Promise<void> => {
  const response = await api.serverRequest(serverWsUrl.value, method, path, body)
  const result = response.body as Partial<ShareLinks> & { statusMessage?: string }

  if (response.status >= 400) throw new Error(result?.statusMessage ?? `The server answered ${response.status}`)
  if (links.value) links.value = { ...links.value, ...result }
}

export function useShareLink(): {
  links: typeof links
  shareUrl: typeof shareUrl
  reset: () => Promise<void>
  setName: (name: string | null) => Promise<void>
  setOfflineMessage: (message: string | null) => Promise<void>
} {
  return {
    links,
    shareUrl,
    reset: () => send('POST', '/api/share-link/reset'),
    setName: (name) => send('PUT', '/api/share-link/name', { name }),
    setOfflineMessage: (message) => send('PUT', '/api/share-link/away', { message })
  }
}
