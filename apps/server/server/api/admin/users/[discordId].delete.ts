import { logAdminAction } from '~~/server/db/adminLog'
import { sendMessageToHost } from '~~/server/routes/host'
import { revokeAccess } from '~~/server/utils/ai/access'

export default defineEventHandler(async (event) => {
  const { discordId: adminId } = await requireAdmin(event)

  const discordId = getRouterParam(event, 'discordId')

  if (!discordId) throw createError({ statusCode: 400, statusMessage: 'Missing discordId' })

  await revokeAccess(discordId)
  sendMessageToHost(discordId, 'ai-access-update', { hasAccess: false, canSelectModel: false })

  await logAdminAction(adminId, 'revoke-ai', discordId, null)

  return { success: true }
})
