// Read-only summary of a room's live state - shared by the share page's SEO meta endpoint
// (server/api/share/[shareId].get.ts) and the link-preview image drawn from the same state
// (server/routes/og/share/[shareId].ts), so the embed's text and picture never disagree.
import { eq } from 'drizzle-orm'

import { getDb } from '~~/server/db'
import { users } from '~~/server/db/schema'
import { hostControls, hostList, hostTheme } from '~~/server/routes/host'

export interface ShareInfo {
  online: boolean
  hostName: string | null
  controlCount: number
  // So the share page renders in the host's colors during SSR instead of switching once the WS sends them.
  theme: { primary: string | null; secondary: string | null } | null
  // While offline: the host's message for viewers, and when they were last online (Unix ms).
  offlineMessage: string | null
  lastOnlineAt: number | null
}

// Takes the room (the host's Discord id), not the code in the link: see db/shareLinks.ts.
export async function getShareInfo(roomId: string): Promise<ShareInfo> {
  const host = hostList.get(roomId)

  if (!host) {
    const [user] = await getDb()
      .select({ name: users.displayName, username: users.username, offlineMessage: users.offlineMessage, lastOnlineAt: users.lastOnlineAt })
      .from(users)
      .where(eq(users.discordId, roomId))
      .limit(1)

    return {
      online: false,
      hostName: user?.name || user?.username || null,
      controlCount: 0,
      theme: null,
      offlineMessage: user?.offlineMessage ?? null,
      lastOnlineAt: user?.lastOnlineAt?.getTime() ?? null
    }
  }

  const controlCount = (hostControls.get(roomId) ?? []).reduce(
    (sum, group) => sum + group.controls.length,
    0
  )

  return {
    online: true,
    // Mirrors the same discord.name -> username fallback used client-side for the same reason
    // (`global_name` can be unset) - see apps/client's useWebsocketHost.ts.
    hostName: host.user.discord?.name || host.user.username,
    controlCount,
    theme: hostTheme.get(roomId) ?? null,
    offlineMessage: null,
    lastOnlineAt: null
  }
}
