import { api } from '@renderer/lib/tauri-bridge'
import { ref } from 'vue'

// OSC over UDP has no handshake, so there's no literal "connected" event to listen for — VRChat
// only ever sends us packets when something changes (avatar params, tracking, /avatar/change...).
// "Connected" here means "we've seen a packet recently"; if that goes quiet for longer than this
// window we flip to disconnected. Starts disconnected until the first packet actually arrives.
const IDLE_TIMEOUT_MS = 8000

const lastMessageAt = ref<number | null>(null)
const connected = ref(false)

function markAlive(): void {
  lastMessageAt.value = Date.now()
  connected.value = true
}

api.onOscMessage(markAlive)
api.onOscMessageBulk(markAlive)

setInterval(() => {
  if (lastMessageAt.value !== null && Date.now() - lastMessageAt.value > IDLE_TIMEOUT_MS) {
    connected.value = false
  }
}, 1000)

export function useOscConnection(): {
  connected: typeof connected
  lastMessageAt: typeof lastMessageAt
} {
  return { connected, lastMessageAt }
}
