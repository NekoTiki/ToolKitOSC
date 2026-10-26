// A viewer's Discord avatar, proxied so the share page never sees the CDN URL (it contains their
// Discord id). See viewerAvatars in routes/ws/[ws].ts.
import { viewerAvatars } from '~~/server/routes/ws/[ws]'

export default defineEventHandler(async (event) => {
  const url = viewerAvatars.get(getRouterParam(event, 'id') ?? '')

  if (!url) {
    throw createError({ statusCode: 404, statusMessage: 'Not Found' })
  }

  const response = await fetch(url)

  if (!response.ok) {
    throw createError({ statusCode: 404, statusMessage: 'Not Found' })
  }

  setResponseHeaders(event, {
    'Content-Type': response.headers.get('content-type') ?? 'image/png',
    'Cache-Control': 'public, max-age=3600'
  })

  return new Uint8Array(await response.arrayBuffer())
})
