// Read-only summary of a room's live state, for the share page's SSR-time SEO meta (see
// app/pages/share/[shareId].vue) - Discord's link-preview crawler doesn't run JS, so the
// personalized title/description has to be in the initial server-rendered HTML, not fetched over
// the WS after mount the way the page's actual control list is.
import { hostControls, hostList } from '~~/server/routes/host'

export interface ShareInfo {
  online: boolean
  hostName: string | null
  controlCount: number
}

export default defineEventHandler((event): ShareInfo => {
  const shareId = getRouterParam(event, 'shareId')

  if (!shareId) {
    throw createError({ statusCode: 400, statusMessage: 'Missing shareId' })
  }

  const host = hostList.get(shareId)

  if (!host) {
    return { online: false, hostName: null, controlCount: 0 }
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
    controlCount
  }
})
