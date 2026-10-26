import { unsuspend } from '~~/server/db/accounts'
import { logAdminAction } from '~~/server/db/adminLog'

export default defineEventHandler(async (event) => {
  const { discordId: adminId } = await requireAdmin(event)

  const discordId = getRouterParam(event, 'discordId')

  if (!discordId) throw createError({ statusCode: 400, statusMessage: 'Missing discordId' })

  await unsuspend(discordId)

  await logAdminAction(adminId, 'lift-suspension', discordId, null)

  return { success: true }
})
