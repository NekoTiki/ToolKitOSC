import type { Peer } from 'crossws'
import jwt from 'jsonwebtoken'

import type { User } from '#auth-utils'
import type { ControlGroup, HostToServerMessage, OSCArg } from '#shared/types/protocol'
import {
  KNOWN_CONTROL_TYPES,
  MIN_SUPPORTED_PROTOCOL_VERSION,
  PROTOCOL_VERSION
} from '#shared/types/protocol'
import { upsertUserFromAuth } from '~~/server/db/users'
import { clientList, sendToClient, sendToEveryoneInRoom } from '~~/server/routes/ws/[ws]'
import { getAiAccess } from '~~/server/utils/ai/access'
import { recordControlInventory } from '~~/server/utils/controlStats'
import { useWsIp } from '~~/server/utils/useWsIp'

type RoomId = string
type PeerId = string

const authenticateHost = new Map<PeerId, User>()
export const hostList = new Map<RoomId, Host>()
// Note: this stores whole ControlGroup[] trees (each with a nested `controls: ControlType[]`),
// not a flat ControlType[] — the old untyped `Map<RoomId, ControlType[]>` annotation was wrong
// and only went unnoticed because this file had no real type checking on the WS payloads before.
export const hostControls = new Map<RoomId, ControlGroup[]>()
export const hostArgs = new Map<RoomId, Record<string, OSCArg[]>>()
export const hostTheme = new Map<RoomId, { primary: string | null; secondary: string | null }>()

interface Host {
  peer: Peer
  user: User
  ip: string
}

const getRoomId = (peerId: string): string | null => {
  const user = authenticateHost.get(peerId)

  if (user) return user.discord?.id || null

  return null
}

const addClient = (roomId: string, peerId: string, data: Host) => {
  authenticateHost.set(peerId, data.user!)
  hostList.get(roomId)?.peer.close()
  hostList.set(roomId, data)
}

const removeClient = (roomId: string) => {
  authenticateHost.delete(hostList.get(roomId)?.peer.id || '')
  hostList?.delete(roomId)
}

export const sendClientListToHost = (roomId: RoomId) => {
  const host = hostList.get(roomId)

  if (!host) return

  host.peer.send({
    type: 'client-list',
    message: Array.from(clientList.get(roomId!)?.values() || []).map((client) => ({
      user: client.user,
      ip: client.ip,
      // `peerId` here must match `from`/the clientList map key, which is the session id
      // (see ws/[ws].ts addClient), not crossws' own ephemeral client.peer.id.
      peerId: client.sessionId
    }))
  })
}

export const sendMessageToHost = (
  roomId: RoomId,
  type: string,
  command: unknown,
  opts?: unknown
) => {
  const host = hostList.get(roomId)

  if (!host) return

  host.peer.send({
    type,
    message: command,
    ...opts!
  })
}

export default defineWebSocketHandler({
  async message(_peer, message) {
    const peer: Peer = _peer as unknown as Peer

    if (message.text() === 'ping') {
      peer.send('pong')
      return
    }

    const roomId = getRoomId(peer.id)
    const data = JSON.parse(message.text()) as HostToServerMessage

    if (data.type === 'auth-token') {
      const { protocolVersion } = data.message

      // Checked before the token itself: a client outside the supported range needs a clear
      // "please update" signal, not a confusing "invalid token" (auth-error), and there's no
      // point spending a JWT verification on a message this server won't service anyway.
      if (
        typeof protocolVersion !== 'number' ||
        protocolVersion < MIN_SUPPORTED_PROTOCOL_VERSION ||
        protocolVersion > PROTOCOL_VERSION
      ) {
        peer.send({
          type: 'protocol-mismatch',
          message: {
            reason: `Client protocol version ${protocolVersion ?? 'unknown'} is not supported by this server (supported: ${MIN_SUPPORTED_PROTOCOL_VERSION}-${PROTOCOL_VERSION})`,
            serverVersion: PROTOCOL_VERSION,
            minSupportedVersion: MIN_SUPPORTED_PROTOCOL_VERSION
          }
        })
        peer.close()
        return
      }

      if (!data.message.token) {
        peer.send({ type: 'auth-error', message: 'Missing authentication token' })
        peer.close()
        return
      }

      const runtimeConfig = useRuntimeConfig()

      try {
        const { user } = jwt.verify(
          data.message.token,
          runtimeConfig.session.password
        ) as jwt.JwtPayload & { sub: string; user: User }

        const roomId = user.discord?.id

        await upsertUserFromAuth(user)
        const aiAccess = await getAiAccess(roomId!)

        peer.send({
          type: 'auth-success',
          message: {
            message: 'Authentication successful',
            serverVersion: PROTOCOL_VERSION,
            supportedControlTypes: KNOWN_CONTROL_TYPES,
            aiAccess
          }
        })

        const userIp = useWsIp(peer)

        addClient(roomId!, peer.id, { peer, user, ip: userIp })

        sendClientListToHost(roomId!)
        sendToEveryoneInRoom(roomId!, { type: 'host-status', message: 'online' })
      } catch (error) {
        console.error('Authentication failed:', error)
        peer.send({ type: 'auth-error', message: 'Invalid authentication token' })

        peer.close()
      }
    } else if (!roomId) {
      // Everything below acts on the host's room - ignore it from a peer that never authenticated
      // (it used to run with an undefined room, e.g. recording inventory under a NULL discord id).
      return
    } else if (data.type === 'controls-update') {
      const { avatarId, groups } = data.message

      hostControls.set(roomId!, groups)
      // Fire-and-forget: this shouldn't add DB latency to relaying the update to every viewer.
      void recordControlInventory(roomId!, avatarId, groups).catch((error) =>
        console.error('[controls] failed to record inventory:', error)
      )

      // Re-sent as just `groups` (ServerToViewerMessage's own 'controls-update' shape never
      // changed) - viewers don't need, and shouldn't rely on, which avatar these came from.
      sendToEveryoneInRoom(roomId!, { type: 'controls-update', message: groups })
    } else if (data.type === 'args-initial') {
      hostArgs.set(roomId!, data.message)

      sendToEveryoneInRoom(roomId!, message.text())
    } else if (data.type === 'args-update') {
      const args = hostArgs.get(roomId!)

      if (!args) {
        hostArgs.set(roomId!, {
          [data.message.address]: data.message.args
        })
      } else args[data.message.address] = data.message.args

      sendToEveryoneInRoom(roomId!, message.text())
    } else if (data.type === 'open-shock-value-update') {
      sendToEveryoneInRoom(roomId!, message.text())
    } else if (data.type === 'intiface-value-update') {
      sendToEveryoneInRoom(roomId!, message.text())
    } else if (data.type === 'intiface-pattern-value-update') {
      sendToEveryoneInRoom(roomId!, message.text())
    } else if (data.type === 'client-invalid') {
      sendToClient(roomId!, data.message.peerId, message.text())
    } else if (data.type === 'client-banned') {
      sendToClient(roomId!, data.message.peerId, message.text())
    } else if (data.type === 'rate-limited') {
      sendToClient(roomId!, data.message.peerId, message.text())
    } else if (data.type === 'theme-update') {
      hostTheme.set(roomId!, data.message)
      sendToEveryoneInRoom(roomId!, message.text())
    }
  },
  close(peer) {
    const roomId = getRoomId(peer.id)

    authenticateHost.delete(peer.id)

    // Only the room's current host going away takes it offline - a connection that was already
    // replaced by a newer one (addClient closes the old peer) must not remove its successor.
    if (!roomId || hostList.get(roomId)?.peer.id !== peer.id) return

    removeClient(roomId)
    sendToEveryoneInRoom(roomId, { type: 'host-status', message: 'offline' })
  },
  error(peer, error) {
    console.error('[ws] error', peer, error)
  }
})
