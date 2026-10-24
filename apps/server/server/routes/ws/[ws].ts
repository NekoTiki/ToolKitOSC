import { createHmac } from 'node:crypto'

import type { Peer } from 'crossws'

import type { User } from '#auth-utils'
import type { ControlGroup, ControlType, PresenceEntry, ViewerToServerMessage } from '#shared/types/protocol'
import { resolveControlForViewer } from '#shared/utils/controlLimits'
import { guestName } from '#shared/utils/guestName'
import { isSuspended } from '~~/server/db/accounts'
import { resolveShareCode } from '~~/server/db/shareLinks'
import {
  hostArgs,
  hostControls,
  hostLastAction,
  hostList,
  hostPaused,
  hostShareSettings,
  hostTheme,
  hostViewerGroups,
  sendClientListToHost,
  sendMessageToHost
} from '~~/server/routes/host'
import { connectionRefusal } from '~~/server/utils/connectionCaps'
import { roomsChanged } from '~~/server/utils/liveRooms'
import { AUTHENTICATED_MAX_PER_WINDOW, clearRateLimit, isRateLimited } from '~~/server/utils/rateLimiter'
import { currentNotice } from '~~/server/utils/serverNotice'
import { getSettings } from '~~/server/utils/serverSettings'
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
  lastActiveAt?: number
}

// Opaque id other viewers see instead of the session id (see PresenceEntry). One per Discord
// account, so someone signed in on two devices is one viewer; one per session for a guest. Keyed,
// because a Discord id is mostly a timestamp: a plain hash of it can be brute-forced back.
const publicIdOf = (client: Client): string =>
  createHmac('sha256', useRuntimeConfig().session.password)
    .update(client.user?.discord?.id ? `discord:${client.user.discord.id}` : client.sessionId)
    .digest('hex')
    .slice(0, 16)

// Same priority as the desktop app's describeViewer (apps/client's useClientsDb.ts), so a viewer
// has the same name on the share page as in the streamer's Viewers page.
const displayNameOf = (client: Client): string =>
  client.user?.discord?.name || client.user?.username || client.user?.userDefinedDisplayName || guestName(client.sessionId)

export const showViewers = (roomId: RoomId): boolean => hostShareSettings.get(roomId)?.showViewers ?? true

// Discord avatar URLs by public id, served by routes/avatar/[id].get.ts: the URL itself contains
// the Discord id, so viewers only ever see /avatar/<public id>.
// shortcut: never pruned, one short string per signed-in viewer seen since start; prune on leave if it grows.
export const viewerAvatars = new Map<string, string>()

export const presenceOf = (client: Client): PresenceEntry => {
  const id = publicIdOf(client)
  const avatar = client.user?.discord?.avatar

  if (avatar) viewerAvatars.set(id, avatar)

  return {
    id,
    name: displayNameOf(client),
    avatar: avatar ? `/avatar/${id}` : undefined,
    discord: !!client.user?.discord?.id,
    lastActiveAt: client.lastActiveAt
  }
}

// One entry per viewer: a Discord account's sessions are merged, keeping its latest activity.
const roomPresence = (roomId: RoomId): PresenceEntry[] | null => {
  if (!showViewers(roomId)) return null

  const entries = new Map<string, PresenceEntry>()

  for (const client of clientList.get(roomId)?.values() ?? []) {
    const entry = presenceOf(client)
    const seen = entries.get(entry.id)

    if (!seen || (entry.lastActiveAt ?? 0) > (seen.lastActiveAt ?? 0)) entries.set(entry.id, entry)
  }

  return Array.from(entries.values())
}

// Sends each viewer the room's list, with their own entry marked. Called on join, leave, rename,
// and when the host changes whether viewers may see each other.
export const broadcastPresence = (roomId: RoomId) => {
  const viewers = roomPresence(roomId)

  for (const client of clientList.get(roomId)?.values() ?? []) {
    for (const peer of client.peers.values()) {
      peer.send({ type: 'presence', message: { you: publicIdOf(client), viewers } })
    }
  }
}

// The room behind the code in /ws/<code>, looked up once per connection: every handler awaits
// the same promise, so a message that arrives while open is still resolving waits for it.
const getRoomId = (peer: { context: Record<string, unknown>; request: { url: string } }): Promise<string | null> => {
  peer.context.roomId ??= resolveShareCode(new URL(peer.request.url).pathname.split('/')[2] ?? '')

  return peer.context.roomId as Promise<string | null>
}

const addClient = (roomId: RoomId, peerId: PeerId, peer: Peer, data: Omit<Client, 'peers'>) => {
  if (!clientList.has(roomId)) {
    clientList.set(roomId, new Map())
  }

  const room = clientList.get(roomId)!
  const existing = room.get(peerId)

  // Another tab of a session that's already here: same viewer, one more connection.
  if (existing) {
    existing.peers.set(peer.id, peer)
    existing.user = data.user
    peer.send({ type: 'presence', message: { you: publicIdOf(existing), viewers: roomPresence(roomId) } })
    return
  }

  room.set(peerId, { ...data, peers: new Map([[peer.id, peer]]) })

  sendClientListToHost(roomId)
  broadcastPresence(roomId)
  roomsChanged()
}

const removeClient = (roomId: RoomId, peerId: PeerId, peer: Pick<Peer, 'id'>) => {
  const client = clientList.get(roomId)?.get(peerId)

  if (!client) return

  client.peers.delete(peer.id)

  if (client.peers.size) return

  clientList.get(roomId)!.delete(peerId)

  sendClientListToHost(roomId)
  broadcastPresence(roomId)
  roomsChanged()
}

export const sendToEveryoneInRoom = (roomId: RoomId, data: unknown) => {
  for (const { peers } of clientList.get(roomId)?.values() ?? []) {
    for (const peer of peers.values()) peer.send(data)
  }
}

// A suspended account's open share pages carry on as a guest, like their next visit will.
export const demoteViewerAccount = (discordId: string) => {
  for (const [roomId, room] of clientList) {
    const clients = [...room.values()].filter((client) => client.user?.discord?.id === discordId)

    if (!clients.length) continue

    for (const client of clients) client.user = null
    sendClientListToHost(roomId)
    broadcastPresence(roomId)
  }
}

export const sendToClient = (roomId: RoomId, peerId: PeerId, data: unknown) => {
  for (const peer of clientList.get(roomId)?.get(peerId)?.peers.values() ?? []) peer.send(data)
}

const groupIdsOf = (roomId: RoomId, sessionId: string): string[] => hostViewerGroups.get(roomId)?.members[sessionId] ?? []

// One control as this viewer may use it: the active profile's rules, opened up by their groups.
const viewerControl = (roomId: RoomId, sessionId: string, control: ControlType): ControlType =>
  resolveControlForViewer(control, groupIdsOf(roomId, sessionId), hostViewerGroups.get(roomId)?.groups)

const viewerControlsFor = (roomId: RoomId, sessionId: string): ControlGroup[] | undefined =>
  hostControls.get(roomId)?.map((group) => ({
    ...group,
    controls: group.controls.map((control) => viewerControl(roomId, sessionId, control))
  }))

// Sends every viewer their own copy of the controls. Viewers in the same groups share one.
export const sendControlsToRoom = (roomId: RoomId) => {
  if (!hostControls.has(roomId)) return

  const copies = new Map<string, ControlGroup[] | undefined>()

  for (const client of clientList.get(roomId)?.values() ?? []) {
    const key = [...groupIdsOf(roomId, client.sessionId)].sort().join(',')

    if (!copies.has(key)) copies.set(key, viewerControlsFor(roomId, client.sessionId))

    for (const peer of client.peers.values()) peer.send({ type: 'controls-update', message: copies.get(key) })
  }
}

export default defineWebSocketHandler({
  async open(_peer) {
    const peer: Peer = _peer as unknown as Peer

    const roomId = await getRoomId(peer)

    // A reset or made-up link: the page already shows a 404 for it.
    if (!roomId) {
      peer.close(4404, 'Not Found')
      return
    }

    const { id: sessionId, user: sessionUser, secure } = await getUserSession(peer)
    const userIp = useWsIp(peer)
    // A suspended account watches as a guest.
    const user = sessionUser?.discord?.id && (await isSuspended(sessionUser.discord.id)) ? null : sessionUser
    const refusal = connectionRefusal({
      socketsFromIp: [...clientList.values()].flatMap((room) => [...room.values()]).filter((client) => client.ip === userIp).reduce((sum, client) => sum + client.peers.size, 0),
      viewersInRoom: clientList.get(roomId)?.size ?? 0,
      alreadyInRoom: !!clientList.get(roomId)?.has(sessionId)
    }, { perIp: getSettings().maxSocketsPerIp, perRoom: getSettings().maxViewersPerRoom })

    if (refusal) {
      peer.send({ type: 'connection-refused', message: { reason: refusal } })
      peer.close(4429, refusal)
      return
    }

    addClient(roomId, sessionId, peer, { sessionId, user: user ?? null, ip: userIp, guestId: secure?.guestId })

    const controls = viewerControlsFor(roomId, sessionId)
    const args = hostArgs.get(roomId)!
    const theme = hostTheme.get(roomId)

    if (controls) {
      peer.send({ type: 'controls-update', message: controls })
    }

    if (args) {
      peer.send({ type: 'args-initial', message: args })
    }

    if (theme) {
      peer.send({ type: 'theme-update', message: theme })
    }

    peer.send({ type: 'host-status', message: hostList.has(roomId) ? 'online' : 'offline' })
    peer.send({ type: 'host-paused', message: hostPaused.has(roomId) })

    peer.send({
      type: 'welcome',
      message: `Welcome ${user?.discord?.name || 'Anonymous'}! Your peer ID is ${sessionId}`
    })

    const notice = currentNotice()

    if (notice?.onSharePages) peer.send({ type: 'server-notice', message: notice })

    const lastAction = hostLastAction.get(roomId)

    if (lastAction) {
      peer.send({ type: 'viewer-action', message: showViewers(roomId) ? lastAction : { ...lastAction, viewer: null } })
    }
  },
  async message(peer, message) {
    if (message.text() === 'ping') {
      peer.send('pong')
      return
    }

    const roomId = await getRoomId(peer)

    if (!roomId) return

    const { id: sessionId, user } = await getUserSession(peer)

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
        // The host stopped everything: the page shows a paused card, and the host would reject it.
        if (hostPaused.has(roomId)) return

        // Only the shape is checked here. Whether this viewer may do this (locks, limits, their
        // groups' rules, sign-in, bans) is decided by the desktop app alone
        // (apps/client/src-tauri/src/server/viewer_commands.rs), which clamps the value and answers
        // with command-accepted. Viewers and the stats only ever see what it accepted.
        if (typeof data.message?.groupId !== 'string' || typeof data.message?.controlId !== 'string' || typeof data.message?.type !== 'string') return
      } else {
        // Mirror the host's copy (see useWebsocketHost.ts), so the new name shows in everyone's list.
        const client = clientList.get(roomId)?.get(sessionId)
        const displayName = typeof data.message.displayName === 'string' ? data.message.displayName.trim().slice(0, 64) : ''

        if (client && displayName) {
          client.user = { ...(client.user ?? { username: '', email: '' }), userDefinedDisplayName: displayName }
          broadcastPresence(roomId)
        }
      }

      sendMessageToHost(roomId, data.type, data.message, {
        from: sessionId
      })
    }
  },
  async close(peer) {
    const roomId = await getRoomId(peer)

    if (!roomId) return

    const { id: sessionId } = await getUserSession(peer)

    removeClient(roomId, sessionId, peer)
    clearRateLimit(`${roomId}:${sessionId}`)
  },
  error(peer, error) {
    console.error('[ws] error', peer, error)
  }
})
