// Read-only summary of a room's live state, for the share page's SSR-time SEO meta (see
// app/pages/share/[shareId].vue) - Discord's link-preview crawler doesn't run JS, so the
// personalized title/description has to be in the initial server-rendered HTML, not fetched over
// the WS after mount the way the page's actual control list is.
import { resolveShareCode } from '~~/server/db/shareLinks'
import type { ShareInfo } from '~~/server/utils/shareInfo'
import { getShareInfo } from '~~/server/utils/shareInfo'

export type { ShareInfo }

export default defineEventHandler(async (event): Promise<ShareInfo> => {
  const shareId = getRouterParam(event, 'shareId')

  if (!shareId) {
    throw createError({ statusCode: 400, statusMessage: 'Missing shareId' })
  }

  const roomId = await resolveShareCode(shareId)

  // The page shows a 404 for this: a reset or made-up link.
  if (!roomId) {
    throw createError({ statusCode: 404, statusMessage: 'Not Found' })
  }

  return await getShareInfo(roomId)
})
