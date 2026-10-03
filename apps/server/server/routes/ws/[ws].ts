import type { Peer } from 'crossws'

import type { User } from '#auth-utils'
import type { ViewerToServerMessage } from '#shared/types/protocol'
import { sanitizeCommand } from '#shared/utils/controlLimits'
import {
  hostArgs,
  hostControls,
  hostList,
  hostTheme,
  sendClientListToHost,
  sendMessageToHost
} from '~~/server/routes/host'
import { scheduleControlActivation } from '~~/server/utils/controlStats'
import { AUTHENTICATED_MAX_PER_WINDOW, clearRateLimit, isRateLimited } from '~~/server/utils/rateLimiter'
import { useWsIp } from '~~/server/utils/useWsIp'

type RoomId = string
type PeerId = string

export const clientList = new Map<RoomId, Map<PeerId, Client>>()

interface Client {
  peer: Peer
  sessionId: string
  user: User | null
  ip: string
  guestId?: string
}

const getRoomId = (url: string): string | null => {
  const path = new URL(url).pathname
  const pathParts = path.split('/')

  if (pathParts.length >= 3) {
    return pathParts[2] ?? null
  }

  return null
}

const addClient = (roomId: RoomId, peerId: PeerId, data: Client) => {
  if (!clientList.has(roomId)) {
    clientList.set(roomId, new Map())
  }

  clientList.get(roomId)!.set(peerId, data)

  sendClientListToHost(roomId)
}

const removeClient = (roomId: RoomId, peerId: PeerId) => {
  clientList.get(roomId)?.delete(peerId)

  sendClientListToHost(roomId)
}

export const sendToEveryoneInRoom = (roomId: RoomId, data: unknown) => {
  for (const [_, { peer }] of clientList.get(roomId)?.entries() ?? []) {
    peer.send(data)
  }
}

export const sendToClient = (roomId: RoomId, peerId: PeerId, data: unknown) => {
  clientList.get(roomId)?.get(peerId)?.peer.send(data)
}

export default defineWebSocketHandler({
  async open(_peer) {
    const peer: Peer = _peer as unknown as Peer

    const { id: sessionId, user, secure } = await getUserSession(peer)

    const roomId = getRoomId(peer.request.url)
    const userIp = useWsIp(peer)

    addClient(roomId!, sessionId, { peer, sessionId, user: user ?? null, ip: userIp, guestId: secure?.guestId })

    const controls = hostControls.get(roomId!)
    const args = hostArgs.get(roomId!)!
    const theme = hostTheme.get(roomId!)

    if (controls) {
      peer.send({ type: 'controls-update', message: Array.from(controls) })
    }

    if (args) {
      peer.send({ type: 'args-initial', message: args })
    }

    if (theme) {
      peer.send({ type: 'theme-update', message: theme })
    }

    peer.send({ type: 'host-status', message: hostList.has(roomId!) ? 'online' : 'offline' })

    peer.send({
      type: 'welcome',
      message: `Welcome ${user?.discord?.name || 'Anonymous'}! Your peer ID is ${sessionId} to the room ${roomId}`
    })
  },
  async message(peer, message) {
    if (message.text() === 'ping') {
      peer.send('pong')
      return
    }

    const { id: sessionId, user } = await getUserSession(peer)

    const roomId = getRoomId(peer.request.url)

    const data = JSON.parse(message.text()) as ViewerToServerMessage

    if (data.type === 'command' || data.type === 'update-username') {
      // Ingress-level rate limit: catches a flooding viewer before it ever reaches the host, on
      // top of (not instead of) the host's own per-executed-command limit (see apps/client's
      // useWebsocketHost.ts) - the host's limit only sees commands that got this far and passed
      // ban/validation checks, so this is what actually protects the relay itself. Deliberately no
      // added delay/throttle here beyond this immediate check - a dragged slider's own displayed
      // position depends on this exact round trip (see ControlSlider.vue's `get()`), so anything
      // that buffers a message before forwarding it makes dragging feel laggy for whoever's
      // holding it. The slider's outgoing rate is instead kept low at the true source (see that
      // component's own debounce), not sampled down again on the way through.
      const maxPerWindow = user?.discord?.id ? AUTHENTICATED_MAX_PER_WINDOW : undefined

      if (isRateLimited(`${roomId}:${sessionId}`, maxPerWindow)) {
        peer.send({
          type: 'rate-limited',
          message: { peerId: sessionId, reason: 'You are sending messages too quickly - slow down.' }
        })
        return
      }

      if (data.type === 'command') {
        // Check the command against the controls this room's viewers were sent (locks, the active
        // profile's limits, value types) so a hand-written message never reaches the host or the
        // stats. A control the cache doesn't know is passed on as-is: the host decides, and logs it.
        const control = hostControls
          .get(roomId!)
          ?.find((group) => group.id === data.message.groupId)
          ?.controls.find((c) => c.id === data.message.controlId)

        if (control) {
          const command = sanitizeCommand(control, data.message)

          if (!command) return

          data.message = { ...data.message, ...command }
        }

        scheduleControlActivation(roomId!, data.message.groupId, data.message.controlId, data.message.type)
      }

      sendMessageToHost(roomId!, data.type, data.message, {
        from: sessionId
      })
    }
  },
  async close(peer) {
    const { id: sessionId } = await getUserSession(peer)

    const roomId = getRoomId(peer.request.url)

    removeClient(roomId!, sessionId)
    clearRateLimit(`${roomId}:${sessionId}`)
  },
  error(peer, error) {
    console.error('[ws] error', peer, error)
  }
})
