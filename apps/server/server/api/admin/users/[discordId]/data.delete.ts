import { deleteAccountData } from '~~/server/db/accounts'
import { logAdminAction } from '~~/server/db/adminLog'
import { signOutHost } from '~~/server/routes/host'

// Someone asked an admin to remove them. They show up again if they sign in later.
export default defineEventHandler(async (event) => {
  const { discordId: adminId } = await requireAdmin(event)

  const discordId = getRouterParam(event, 'discordId')

  if (!discordId) throw createError({ statusCode: 400, statusMessage: 'Missing discordId' })

  await deleteAccountData(discordId)
  signOutHost(discordId, 'Your data was deleted from this server.')

  await logAdminAction(adminId, 'delete-data', discordId, null)

  return { success: true }
})
