import type { H3Event } from 'h3'

// For the browser-side /dashboard API only (see server/api/admin/*) - requireUserSession is the
// cookie-session gate (nuxt-auth-utils), matching how the rest of the browser app authenticates;
// the desktop client never calls these routes, so verifyDesktopToken doesn't apply here.
export async function requireAdmin(event: H3Event): Promise<{ discordId: string }> {
  const { user } = await requireUserSession(event)

  if (!user.discord?.id || !isAdmin(user.discord.id)) {
    throw createError({ statusCode: 403, statusMessage: 'Admin access required' })
  }

  return { discordId: user.discord.id }
}
