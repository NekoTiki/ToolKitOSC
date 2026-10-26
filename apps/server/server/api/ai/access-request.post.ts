import { z } from 'zod'

import { upsertUserFromAuth } from '~~/server/db/users'
import { notifyAdmins } from '~~/server/routes/admin/ws'
import { requestAccess } from '~~/server/utils/ai/access'

const bodySchema = z.object({ message: z.string().trim().max(300).optional() })

// "Request access" on the app's AI page. Admins see it on the dashboard's Users page.
export default defineEventHandler(async (event) => {
  const user = await verifyDesktopToken(event)

  if (!user.discord?.id) throw createError({ statusCode: 401, statusMessage: 'No Discord identity on token' })

  const { message } = await readValidatedBody(event, bodySchema.parse)

  // The request row points at the user row.
  await upsertUserFromAuth(user)

  if ((await requestAccess(user.discord.id, message || null)) === 'too-soon') {
    throw createError({ statusCode: 429, statusMessage: 'Your last request was declined. You can ask again a week after.' })
  }

  notifyAdmins({ type: 'access-requested' })

  return { status: 'pending' }
})
