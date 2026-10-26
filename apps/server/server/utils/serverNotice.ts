import { randomUUID } from 'node:crypto'

import type { ServerNotice } from '#shared/types/protocol'
import { hostList } from '~~/server/routes/host'
import { clientList } from '~~/server/routes/ws/[ws]'

// shortcut: kept in memory, so a restart clears it; store it if notices need to outlive restarts.
let current: ServerNotice | null = null

export function currentNotice(): ServerNotice | null {
  if (current?.expiresAt && current.expiresAt <= Date.now()) current = null

  return current
}

const broadcast = (notice: ServerNotice | null, toViewers: boolean): void => {
  const payload = { type: 'server-notice', message: notice }

  for (const host of hostList.values()) host.peer.send(payload)

  if (!toViewers) return

  for (const room of clientList.values()) {
    for (const client of room.values()) {
      for (const peer of client.peers.values()) peer.send(payload)
    }
  }
}

export function setNotice(notice: Omit<ServerNotice, 'id'>): ServerNotice {
  // Share pages that showed the old one must drop it even if this one stays off them.
  const shownOnPages = !!currentNotice()?.onSharePages

  current = { ...notice, id: randomUUID() }
  if (shownOnPages && !current.onSharePages) broadcast(null, true)
  broadcast(current, current.onSharePages)

  return current
}

export function clearNotice(): void {
  const shownOnPages = !!currentNotice()?.onSharePages

  current = null
  broadcast(null, shownOnPages)
}
