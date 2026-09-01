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

const token = ref<string | undefined>()

export function useAuth(): {
  loggedIn: ComputedRef<boolean>
  token: typeof token
  user: ComputedRef<User | null>
  setToken: (token?: string) => void
} {
  const loggedIn = computed(() => !!token.value)

  const setToken = (newToken?: string): void => {
    token.value = newToken

    if (newToken) localStorage.setItem('authToken', newToken)
    else localStorage.removeItem('authToken')
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

  onMounted(() => (token.value = localStorage.getItem('authToken') || undefined))

  return { loggedIn, token, user, setToken }
}
