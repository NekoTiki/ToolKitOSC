import { logAdminAction } from '~~/server/db/adminLog'
import { hostList, sendMessageToHost } from '~~/server/routes/host'

// Presses the host's Stop everything for them (something going wrong on a stream they can't see).
// Their app stops and pauses as if they pressed it, then says so with pause-state; only they can resume.
export default defineEventHandler(async (event) => {
  const { discordId: adminId } = await requireAdmin(event)

  const discordId = getRouterParam(event, 'discordId')

  if (!discordId) throw createError({ statusCode: 400, statusMessage: 'Missing discordId' })

  if (!hostList.has(discordId)) throw createError({ statusCode: 409, statusMessage: 'The host is offline' })

  sendMessageToHost(discordId, 'admin-stop', null)

  await logAdminAction(adminId, 'stop-everything', discordId, null)

  return { success: true }
})
