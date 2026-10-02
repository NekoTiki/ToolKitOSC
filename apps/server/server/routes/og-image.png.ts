// Generic site link-preview image (no room) - drawn by server/utils/ogImage.ts like the per-room
// ones, at the URL the old static public/og-image.png lived at, so embeds cached against it still
// resolve. It never changes, so it's rendered once per process.
import { renderOgImage } from '~~/server/utils/ogImage'

let png: Promise<Buffer> | null = null

export default defineEventHandler(async (event) => {
  png ??= renderOgImage({ share: null })

  setResponseHeaders(event, {
    'Content-Type': 'image/png',
    'Cache-Control': 'public, max-age=86400'
  })

  return await png
})
