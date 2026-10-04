import { createHash } from 'node:crypto'

import type { Peer } from 'crossws'

import type { User } from '#auth-utils'
import type { PresenceEntry, ViewerToServerMessage } from '#shared/types/protocol'
import { sanitizeCommand } from '#shared/utils/controlLimits'
import { guestName } from '#shared/utils/guestName'
import {
  hostArgs,
  hostControls,
  hostLastAction,
  hostList,
  hostShareSettings,
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
  // Every open tab of this session, by crossws peer id. The client stays listed until the last one
  // closes.
  peers: Map<string, Peer>
  sessionId: string
  user: User | null
  ip: string
  guestId?: string
  // Opaque id other viewers see instead of the session id (see PresenceEntry).
  publicId: string
  lastActiveAt?: number
}

const publicIdOf = (sessionId: string): string => createHash('sha256').update(sessionId).digest('hex').slice(0, 16)

// Same priority as the desktop app's describeViewer (apps/client's useClientsDb.ts), so a viewer
// has the same name on the share page as in the streamer's Viewers page.
const displayNameOf = (client: Client): string =>
  client.user?.discord?.name || client.user?.username || client.user?.userDefinedDisplayName || guestName(client.sessionId)

export const showViewers = (roomId: RoomId): boolean => hostShareSettings.get(roomId)?.showViewers ?? true

export const presenceOf = (client: Client): PresenceEntry => ({
  id: client.publicId,
  name: displayNameOf(client),
  avatar: client.user?.discord?.avatar,
  discord: !!client.user?.discord?.id,
  lastActiveAt: client.lastActiveAt
})

const roomPresence = (roomId: RoomId): PresenceEntry[] | null =>
  showViewers(roomId) ? Array.from(clientList.get(roomId)?.values() ?? [], presenceOf) : null

// Sends each viewer the room's list, with their own entry marked. Called on join, leave, rename,
// and when the host changes whether viewers may see each other.
export const broadcastPresence = (roomId: RoomId) => {
  const viewers = roomPresence(roomId)

  for (const client of clientList.get(roomId)?.values() ?? []) {
    for (const peer of client.peers.values()) {
      peer.send({ type: 'presence', message: { you: client.publicId, viewers } })
    }
  }
}

const getRoomId = (url: string): string | null => {
  const path = new URL(url).pathname
  const pathParts = path.split('/')

  if (pathParts.length >= 3) {
    return pathParts[2] ?? null
  }

  return null
}

const addClient = (roomId: RoomId, peerId: PeerId, peer: Peer, data: Omit<Client, 'peers' | 'publicId'>) => {
  if (!clientList.has(roomId)) {
    clientList.set(roomId, new Map())
  }

  const room = clientList.get(roomId)!
  const existing = room.get(peerId)

  // Another tab of a session that's already here: same viewer, one more connection.
  if (existing) {
    existing.peers.set(peer.id, peer)
    existing.user = data.user
    peer.send({ type: 'presence', message: { you: existing.publicId, viewers: roomPresence(roomId) } })
    return
  }

  room.set(peerId, { ...data, peers: new Map([[peer.id, peer]]), publicId: publicIdOf(peerId) })

  sendClientListToHost(roomId)
  broadcastPresence(roomId)
}

const removeClient = (roomId: RoomId, peerId: PeerId, peer: Pick<Peer, 'id'>) => {
  const client = clientList.get(roomId)?.get(peerId)

  if (!client) return

  client.peers.delete(peer.id)

  if (client.peers.size) return

  clientList.get(roomId)!.delete(peerId)

  sendClientListToHost(roomId)
  broadcastPresence(roomId)
}

export const sendToEveryoneInRoom = (roomId: RoomId, data: unknown) => {
  for (const { peers } of clientList.get(roomId)?.values() ?? []) {
    for (const peer of peers.values()) peer.send(data)
  }
}

export const sendToClient = (roomId: RoomId, peerId: PeerId, data: unknown) => {
  for (const peer of clientList.get(roomId)?.get(peerId)?.peers.values() ?? []) peer.send(data)
}

export default defineWebSocketHandler({
  async open(_peer) {
    const peer: Peer = _peer as unknown as Peer

    const { id: sessionId, user, secure } = await getUserSession(peer)

    const roomId = getRoomId(peer.request.url)
    const userIp = useWsIp(peer)

    addClient(roomId!, sessionId, peer, { sessionId, user: user ?? null, ip: userIp, guestId: secure?.guestId })

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

    const lastAction = hostLastAction.get(roomId!)

    if (lastAction) {
      peer.send({ type: 'viewer-action', message: showViewers(roomId!) ? lastAction : { ...lastAction, viewer: null } })
    }
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
      } else {
        // Mirror the host's copy (see useWebsocketHost.ts), so the new name shows in everyone's list.
        const client = clientList.get(roomId!)?.get(sessionId)
        const displayName = typeof data.message.displayName === 'string' ? data.message.displayName.trim().slice(0, 64) : ''

        if (client && displayName) {
          client.user = { ...(client.user ?? { username: '', email: '' }), userDefinedDisplayName: displayName }
          broadcastPresence(roomId!)
        }
      }

      sendMessageToHost(roomId!, data.type, data.message, {
        from: sessionId
      })
    }
  },
  async close(peer) {
    const { id: sessionId } = await getUserSession(peer)

    const roomId = getRoomId(peer.request.url)

    removeClient(roomId!, sessionId, peer)
    clearRateLimit(`${roomId}:${sessionId}`)
  },
  error(peer, error) {
    console.error('[ws] error', peer, error)
  }
})
