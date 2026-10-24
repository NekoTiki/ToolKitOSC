import { logAdminAction } from '~~/server/db/adminLog'
import { setCustomShareName } from '~~/server/db/shareLinks'

// Frees a user's custom share link name (squatting, impersonation). Their random link still works.
export default defineEventHandler(async (event) => {
  const { discordId: adminId } = await requireAdmin(event)

  const discordId = getRouterParam(event, 'discordId')

  if (!discordId) throw createError({ statusCode: 400, statusMessage: 'Missing discordId' })

  await setCustomShareName(discordId, null)

  await logAdminAction(adminId, 'free-share-name', discordId, null)

  return { success: true }
})
