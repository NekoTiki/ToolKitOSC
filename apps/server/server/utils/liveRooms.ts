import { mainShareCode } from '~~/server/db/shareLinks'
import { notifyAdmins } from '~~/server/routes/admin/ws'
import { hostLastAction, hostList, hostPaused } from '~~/server/routes/host'
import { clientList } from '~~/server/routes/ws/[ws]'

export interface LiveRoom {
  discordId: string
  name: string | null
  avatar: string | null
  online: boolean
  onlineSince: number | null
  paused: boolean
  viewers: number
  signedInViewers: number
  lastActionAt: number | null
  shareCode: string
}

// Every room with its host online or viewers waiting on the share page, for the dashboard's Live page.
export async function liveRooms(): Promise<LiveRoom[]> {
  const ids = new Set([...hostList.keys(), ...[...clientList].filter(([, clients]) => clients.size).map(([id]) => id)])

  return Promise.all(
    [...ids].map(async (discordId) => {
      const host = hostList.get(discordId)
      const clients = [...(clientList.get(discordId)?.values() ?? [])]

      return {
        discordId,
        name: host?.user.discord?.name ?? null,
        avatar: host?.user.discord?.avatar ?? null,
        online: !!host,
        onlineSince: host?.connectedAt ?? null,
        paused: hostPaused.has(discordId),
        viewers: clients.length,
        signedInViewers: clients.filter((client) => client.user?.discord?.id).length,
        lastActionAt: hostLastAction.get(discordId)?.at ?? null,
        shareCode: await mainShareCode(discordId)
      }
    })
  )
}

let pending: ReturnType<typeof setTimeout> | undefined

// Tells open Live pages to reload, at most once a second: viewers joining in a burst or a dragged
// slider would otherwise send one reload each.
export function roomsChanged(): void {
  pending ??= setTimeout(() => {
    pending = undefined
    notifyAdmins({ type: 'rooms-changed' })
  }, 1000)
}
