import { useAuth } from '@renderer/composables/useAuth'
import { serverWsUrl } from '@renderer/composables/useWebsocketSettings'
import { api } from '@renderer/lib/tauri-bridge'
import type { Ref } from 'vue'
import { ref } from 'vue'

export type AccessRequestStatus = 'none' | 'pending' | 'declined' | 'granted'

// "Request access" on the AI page: asks the server's admins for AI suggestions. Their answer
// arrives as ai-access-update (see useAiAccess.ts) when they approve.
export function useAiAccessRequest(): {
  status: Ref<AccessRequestStatus | null>
  error: Ref<string | null>
  sending: Ref<boolean>
  load: () => Promise<void>
  send: (message: string) => Promise<void>
} {
  const { loggedIn } = useAuth()
  const status = ref<AccessRequestStatus | null>(null)
  const error = ref<string | null>(null)
  const sending = ref(false)

  const load = async (): Promise<void> => {
    if (!loggedIn.value) return

    try {
      const response = await api.serverRequest(serverWsUrl.value, 'GET', '/api/ai/access-request')

      // An older server without requests: the page keeps its plain "ask an admin" text.
      if (response.status < 400) status.value = (response.body as { status: AccessRequestStatus }).status
    } catch (err) {
      console.error('Could not check the AI access request:', err)
    }
  }

  const send = async (message: string): Promise<void> => {
    sending.value = true
    error.value = null

    try {
      const response = await api.serverRequest(serverWsUrl.value, 'POST', '/api/ai/access-request', { message: message.trim() || undefined })

      if (response.status < 400) status.value = 'pending'
      else error.value = (response.body as { statusMessage?: string } | null)?.statusMessage ?? `The server refused the request (${response.status}).`
    } catch (err) {
      error.value = String(err)
    } finally {
      sending.value = false
    }
  }

  return { status, error, sending, load, send }
}
