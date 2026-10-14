import { useAuth } from '@renderer/composables/useAuth'
import { serverWsUrl } from '@renderer/composables/useWebsocketSettings'
import { api } from '@renderer/lib/tauri-bridge'
import { ref, watch } from 'vue'

// The link to open in the browser to sign in, while signed out. Rust runs the server's sign-in
// socket (see src-tauri/src/server/auth.rs) and keeps the token it ends with.
const authUrl = ref<string | null>(null)

api.onAuthUri((uri) => (authUrl.value = uri))

const { ready, loggedIn } = useAuth()

// Signed out: ask the current server for a sign-in link, again after a server switch. Rust keeps
// one sign-in at a time, so a repeat just sends the link it has. Module scope, so it keeps running
// whichever page is open.
watch(
  [ready, loggedIn, serverWsUrl],
  ([known, signedIn, url]) => {
    if (!known) return
    if (signedIn) void api.authSignInCancel()
    else void api.authSignIn(url)
  },
  { immediate: true }
)

export function useWebsocketAuth(): {
  authUrl: typeof authUrl
} {
  return { authUrl }
}
