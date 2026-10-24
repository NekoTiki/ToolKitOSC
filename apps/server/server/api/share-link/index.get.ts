import { eq } from 'drizzle-orm'

import { getDb } from '~~/server/db'
import { users } from '~~/server/db/schema'
import { customShareName, mainShareCode } from '~~/server/db/shareLinks'

// The host's own share link codes, for the app's Settings > Sharing and the connect page.
export default defineEventHandler(async (event) => {
  const discordId = await shareLinkOwner(event)

  const [user] = await getDb().select({ offlineMessage: users.offlineMessage }).from(users).where(eq(users.discordId, discordId)).limit(1)

  return { main: await mainShareCode(discordId), custom: await customShareName(discordId), offlineMessage: user?.offlineMessage ?? null }
})
