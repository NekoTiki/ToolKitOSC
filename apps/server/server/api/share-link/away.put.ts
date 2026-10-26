import { eq } from 'drizzle-orm'
import { z } from 'zod'

import { getDb } from '~~/server/db'
import { users } from '~~/server/db/schema'

const bodySchema = z.object({ message: z.string().trim().max(200).nullable() })

// The host's message on the share page while they're offline ("Back Friday 9 pm"). Saved on the
// server, so it shows even when the app is closed. Empty or null clears it.
export default defineEventHandler(async (event) => {
  const discordId = await shareLinkOwner(event)
  const { message } = await readValidatedBody(event, bodySchema.parse)

  await getDb()
    .update(users)
    .set({ offlineMessage: message || null })
    .where(eq(users.discordId, discordId))

  return { offlineMessage: message || null }
})
