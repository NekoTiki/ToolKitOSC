import { logAdminAction } from '~~/server/db/adminLog'
import { clearNotice } from '~~/server/utils/serverNotice'

export default defineEventHandler(async (event) => {
  const { discordId: adminId } = await requireAdmin(event)

  clearNotice()

  await logAdminAction(adminId, 'clear-announcement', null, null)

  return { success: true }
})
