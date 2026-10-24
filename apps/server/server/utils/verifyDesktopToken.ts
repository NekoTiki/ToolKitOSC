import type { H3Event } from 'h3'
import jwt from 'jsonwebtoken'

import { accountProblem } from '~~/server/db/accounts'

interface DesktopUser {
  discord?: { id: string; avatar: string; name: string }
  username: string
  email: string
}

// The desktop client never holds a browser session cookie (see apps/client's useAuth.ts) - it
// authenticates with the same long-lived JWT it already sends over the host WebSocket (see
// routes/host.ts's identical jwt.verify call against the same secret), just carried as a bearer
// token instead of a WS message payload here. Use this for any plain REST route the desktop app
// calls directly, instead of requireUserSession (that's for the browser-side Discord OAuth pages).
export async function verifyDesktopToken(event: H3Event): Promise<DesktopUser> {
  const header = getHeader(event, 'authorization')
  const token = header?.startsWith('Bearer ') ? header.slice('Bearer '.length) : undefined

  if (!token) throw createError({ statusCode: 401, statusMessage: 'Missing bearer token' })

  const runtimeConfig = useRuntimeConfig()

  let payload: jwt.JwtPayload & { sub: string; user: DesktopUser; tv?: number }

  try {
    payload = jwt.verify(token, runtimeConfig.session.password) as typeof payload
  } catch {
    throw createError({ statusCode: 401, statusMessage: 'Invalid or expired token' })
  }

  // Suspended, or signed out everywhere since this token was made.
  const problem = payload.user.discord?.id ? await accountProblem(payload.user.discord.id, payload.tv ?? 0) : null

  if (problem) throw createError({ statusCode: 401, statusMessage: problem })

  return payload.user
}
