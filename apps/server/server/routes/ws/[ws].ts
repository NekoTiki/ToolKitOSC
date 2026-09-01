import type { Peer } from 'crossws'

import type { User } from '#auth-utils'
import type { ViewerToServerMessage } from '#shared/types/protocol'
import {
  hostArgs,
  hostControls,
  hostList,
  hostTheme,
  sendClientListToHost,
  sendMessageToHost
} from '~~/server/routes/host'
import { useWsIp } from '~~/server/utils/useWsIp'

type RoomId = string
type PeerId = string

export const clientList = new Map<RoomId, Map<PeerId, Client>>()

interface Client {
  peer: Peer
  sessionId: string
  user: User | null
  ip: string
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

    const { id: sessionId, user } = await getUserSession(peer)

    const roomId = getRoomId(peer.request.url)
    const userIp = useWsIp(peer)

    addClient(roomId!, sessionId, { peer, sessionId, user: user ?? null, ip: userIp })

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
    if (message.text().includes('ping')) {
      peer.send('pong')
      return
    }

    const { id: sessionId } = await getUserSession(peer)

    const roomId = getRoomId(peer.request.url)

    const data = JSON.parse(message.text()) as ViewerToServerMessage
    if (data.type === 'command') {
      sendMessageToHost(roomId!, data.type, data.message, {
        from: sessionId
      })
    } else if (data.type === 'update-username') {
      sendMessageToHost(roomId!, data.type, data.message, {
        from: sessionId
      })
    }
  },
  async close(peer) {
    const { id: sessionId } = await getUserSession(peer)

    const roomId = getRoomId(peer.request.url)

    removeClient(roomId!, sessionId)
  },
  error(peer, error) {
    console.error('[ws] error', peer, error)
  }
})
