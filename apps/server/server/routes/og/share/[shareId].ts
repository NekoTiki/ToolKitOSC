// Per-room link-preview image for the share page's og:image/twitter:image (see
// app/pages/share/[shareId].vue): the host's name, live state and theme, drawn by
// server/utils/ogImage.ts from the same state as the embed's title/description.
import { hostTheme } from '~~/server/routes/host'
import { renderOgImage } from '~~/server/utils/ogImage'
import { getShareInfo } from '~~/server/utils/shareInfo'

export default defineEventHandler(async (event) => {
  const shareId = getRouterParam(event, 'shareId')

  if (!shareId) {
    throw createError({ statusCode: 400, statusMessage: 'Missing shareId' })
  }

  const theme = hostTheme.get(shareId)
  const png = await renderOgImage({ share: getShareInfo(shareId), primary: theme?.primary, secondary: theme?.secondary })

  setResponseHeaders(event, {
    'Content-Type': 'image/png',
    // Short: the card shows live state. The page also versions the URL by that state (?v=), so a
    // fresh embed doesn't wait on this anyway - it just keeps a burst of crawlers off the renderer.
    'Cache-Control': 'public, max-age=300'
  })

  return png
})
