import { z } from 'zod'

import { suspend } from '~~/server/db/accounts'
import { logAdminAction } from '~~/server/db/adminLog'
import { signOutHost } from '~~/server/routes/host'
import { demoteViewerAccount } from '~~/server/routes/ws/[ws]'

const bodySchema = z.object({
  // null: until an admin lifts it.
  days: z.number().int().min(1).max(3650).nullable(),
  reason: z.string().trim().max(300).optional()
})

// The app is signed out and can't sign in again until it ends; share pages treat them as a guest.
export default defineEventHandler(async (event) => {
  const { discordId: adminId } = await requireAdmin(event)

  const discordId = getRouterParam(event, 'discordId')

  if (!discordId) throw createError({ statusCode: 400, statusMessage: 'Missing discordId' })

  const { days, reason } = await readValidatedBody(event, bodySchema.parse)
  const message = await suspend(discordId, days, reason || null)

  signOutHost(discordId, message)
  demoteViewerAccount(discordId)

  await logAdminAction(adminId, 'suspend', discordId, { days, reason })

  return { success: true }
})
