import { logAdminAction } from '~~/server/db/adminLog'
import { declineRequest } from '~~/server/utils/ai/access'

// Approving is granting access (POST /api/admin/users), which also clears the request.
export default defineEventHandler(async (event) => {
  const { discordId: adminId } = await requireAdmin(event)

  const discordId = getRouterParam(event, 'discordId')

  if (!discordId) throw createError({ statusCode: 400, statusMessage: 'Missing discordId' })

  await declineRequest(discordId)
  await logAdminAction(adminId, 'decline-ai-request', discordId, null)

  return { success: true }
})
