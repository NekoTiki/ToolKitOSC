import type { TopViewer } from '#shared/types/protocol'
import { hostShareSettings, sendMessageToHost } from '~~/server/routes/host'
import { sendToEveryoneInRoom } from '~~/server/routes/ws/[ws]'

// One stream per room, from the host's app connecting: when it started, the most viewers at once,
// and how many actions each viewer got through. The app shows it under Activity > Stats; viewers
// see the top ones if the host turns that on. A reconnect within STREAM_GAP_MS is the same stream.
// shortcut: in memory, so a server restart starts a new stream.
interface Stream {
  startedAt: number
  endedAt: number | null
  peakViewers: number
  actions: Map<string, TopViewer>
}

const STREAM_GAP_MS = 10 * 60 * 1000
const TOP_COUNT = 5

const streams = new Map<string, Stream>()

const sendStats = (roomId: string, stream: Stream): void =>
  sendMessageToHost(roomId, 'room-stats', { startedAt: stream.startedAt, peakViewers: stream.peakViewers })

export function streamStarted(roomId: string, viewers: number): void {
  const stream = streams.get(roomId)

  if (!stream || (stream.endedAt && Date.now() - stream.endedAt > STREAM_GAP_MS)) {
    streams.set(roomId, { startedAt: Date.now(), endedAt: null, peakViewers: viewers, actions: new Map() })
  } else {
    stream.endedAt = null
    stream.peakViewers = Math.max(stream.peakViewers, viewers)
  }

  sendStats(roomId, streams.get(roomId)!)
}

export function streamEnded(roomId: string): void {
  const stream = streams.get(roomId)

  if (stream) stream.endedAt = Date.now()
}

export function noteViewerCount(roomId: string, viewers: number): void {
  const stream = streams.get(roomId)

  if (!stream || stream.endedAt || viewers <= stream.peakViewers) return

  stream.peakViewers = viewers
  sendStats(roomId, stream)
}

// Shown only while viewers see each other and the host chose to show it.
export function topViewers(roomId: string): TopViewer[] | null {
  const settings = hostShareSettings.get(roomId)

  if (!settings?.showTopViewers || settings.showViewers === false) return null

  return [...(streams.get(roomId)?.actions.values() ?? [])].sort((a, b) => b.count - a.count).slice(0, TOP_COUNT)
}

let pending = new Set<string>()
let timer: ReturnType<typeof setTimeout> | undefined

// Sent to the room at most every 2 seconds, so a dragged slider isn't one message per step.
export function sendTopViewers(roomId: string): void {
  pending.add(roomId)
  timer ??= setTimeout(() => {
    for (const id of pending) sendToEveryoneInRoom(id, { type: 'top-viewers', message: topViewers(id) })
    pending = new Set()
    timer = undefined
  }, 2000)
}

export function noteAction(roomId: string, viewer: Omit<TopViewer, 'count'>): void {
  const stream = streams.get(roomId)

  if (!stream) return

  const seen = stream.actions.get(viewer.id)

  stream.actions.set(viewer.id, { ...viewer, count: (seen?.count ?? 0) + 1 })
  if (hostShareSettings.get(roomId)?.showTopViewers) sendTopViewers(roomId)
}
