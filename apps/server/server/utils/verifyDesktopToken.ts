import type { H3Event } from 'h3'
import jwt from 'jsonwebtoken'

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
export function verifyDesktopToken(event: H3Event): DesktopUser {
  const header = getHeader(event, 'authorization')
  const token = header?.startsWith('Bearer ') ? header.slice('Bearer '.length) : undefined

  if (!token) throw createError({ statusCode: 401, statusMessage: 'Missing bearer token' })

  const runtimeConfig = useRuntimeConfig()

  try {
    const payload = jwt.verify(token, runtimeConfig.session.password) as jwt.JwtPayload & {
      sub: string
      user: DesktopUser
    }

    return payload.user
  } catch {
    throw createError({ statusCode: 401, statusMessage: 'Invalid or expired token' })
  }
}
