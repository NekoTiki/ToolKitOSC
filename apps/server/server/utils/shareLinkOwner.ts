import type { H3Event } from 'h3'

import { upsertUserFromAuth } from '~~/server/db/users'

// The signed-in host for the /api/share-link routes: the desktop app sends its bearer token, the
// website (connect page) its session cookie. Upserted so their share_links rows have a user row.
export async function shareLinkOwner(event: H3Event): Promise<string> {
  const user = getHeader(event, 'authorization') ? verifyDesktopToken(event) : (await requireUserSession(event)).user

  if (!user.discord?.id) throw createError({ statusCode: 401, statusMessage: 'No Discord identity' })

  await upsertUserFromAuth(user)

  return user.discord.id
}
