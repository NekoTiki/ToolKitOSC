// Short link for the share page (GET /s/:code -> redirect to /share/:code). The page takes the
// same code; it's checked here too so a dead link 404s before the redirect.
import { resolveShareCode } from '~~/server/db/shareLinks'

export default defineEventHandler(async (event) => {
  const code = getRouterParam(event, 'code')

  if (!code) {
    throw createError({ statusCode: 400, statusMessage: 'Missing code' })
  }

  if (!(await resolveShareCode(code))) {
    throw createError({ statusCode: 404, statusMessage: 'Not Found' })
  }

  return sendRedirect(event, `/share/${encodeURIComponent(code)}`)
})
