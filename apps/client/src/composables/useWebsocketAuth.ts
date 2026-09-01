import { useAuth } from '@renderer/composables/useAuth'
import { serverWsUrl } from '@renderer/composables/useWebsocketSettings'
import { useWebSocket } from '@vueuse/core'
import { onMounted, ref } from 'vue'

const authUrl = ref<string | null>()

export function useWebsocketAuth(): {
  authUrl: typeof authUrl
} {
  const { loggedIn, setToken } = useAuth()

  const { open, close } = useWebSocket(() => `${serverWsUrl.value}/auth/desktop/ws`, {
    heartbeat: {
      message: 'ping',
      interval: 5000,
      pongTimeout: 5000
    },
    immediate: false,
    onMessage: (_ws, ev) => {
      if (ev.data === 'pong') return

      const data = JSON.parse(ev.data)

      if (data.type === 'auth-uri') {
        authUrl.value = data.message.authUrl
      } else if (data.type === 'auth-token') {
        setToken(data.message.token)
        close()
      }

      console.log(
        `%c[⇩ WS Auth]%c[${data.type}] %c${JSON.stringify(data.message)}`,
        'color: #42b983; font-weight: bold;',
        'color: #ba43ba; font-weight: bold;',
        'color: #ffffff;'
      )
    }
  })

  onMounted(() => !loggedIn.value && open())

  return { authUrl }
}
