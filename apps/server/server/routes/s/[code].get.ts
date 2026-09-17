// Short link for the share page (GET /s/:code -> redirect to /share/:roomId). The room id has
// always been the host's raw Discord id (see server/routes/host.ts's `getRoomId`) - `code` is just
// a reversible base62 re-encoding of that same id (see shared/utils/shortLink.ts), so this needs no
// lookup table of its own: a bad/garbled code simply fails to decode into a valid id.
import { decodeShareCode } from '#shared/utils/shortLink'

export default defineEventHandler((event) => {
  const code = getRouterParam(event, 'code')

  if (!code) {
    throw createError({ statusCode: 400, statusMessage: 'Missing code' })
  }

  const roomId = decodeShareCode(code)

  if (!roomId) {
    throw createError({ statusCode: 404, statusMessage: 'Not Found' })
  }

  return sendRedirect(event, `/share/${roomId}`)
})
