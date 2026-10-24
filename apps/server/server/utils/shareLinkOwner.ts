import type { H3Event } from 'h3'

import { accountProblem } from '~~/server/db/accounts'
import { upsertUserFromAuth } from '~~/server/db/users'

// The signed-in host for the /api/share-link routes: the desktop app sends its bearer token, the
// website (connect page) its session cookie. Upserted so their share_links rows have a user row.
export async function shareLinkOwner(event: H3Event): Promise<string> {
  const user = getHeader(event, 'authorization') ? await verifyDesktopToken(event) : (await requireUserSession(event)).user

  if (!user.discord?.id) throw createError({ statusCode: 401, statusMessage: 'No Discord identity' })

  // The desktop token was checked above; a browser session only for a suspension.
  const problem = await accountProblem(user.discord.id)

  if (problem) throw createError({ statusCode: 403, statusMessage: problem })

  await upsertUserFromAuth(user)

  return user.discord.id
}
