// Read-only summary of a room's live state - shared by the share page's SEO meta endpoint
// (server/api/share/[shareId].get.ts) and the link-preview image drawn from the same state
// (server/routes/og/share/[shareId].ts), so the embed's text and picture never disagree.
import { hostControls, hostList, hostTheme } from '~~/server/routes/host'

export interface ShareInfo {
  online: boolean
  hostName: string | null
  controlCount: number
  // So the share page renders in the host's colors during SSR instead of switching once the WS sends them.
  theme: { primary: string | null; secondary: string | null } | null
}

export function getShareInfo(shareId: string): ShareInfo {
  const host = hostList.get(shareId)

  if (!host) {
    return { online: false, hostName: null, controlCount: 0, theme: null }
  }

  const controlCount = (hostControls.get(shareId) ?? []).reduce(
    (sum, group) => sum + group.controls.length,
    0
  )

  return {
    online: true,
    // Mirrors the same discord.name -> username fallback used client-side for the same reason
    // (`global_name` can be unset) - see apps/client's useWebsocketHost.ts.
    hostName: host.user.discord?.name || host.user.username,
    controlCount,
    theme: hostTheme.get(shareId) ?? null
  }
}
