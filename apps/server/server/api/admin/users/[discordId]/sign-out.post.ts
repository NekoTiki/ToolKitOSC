import { signOutEverywhere } from '~~/server/db/accounts'
import { logAdminAction } from '~~/server/db/adminLog'
import { signOutHost } from '~~/server/routes/host'

// Every desktop token made before now stops working (a leaked token, a shared PC).
export default defineEventHandler(async (event) => {
  const { discordId: adminId } = await requireAdmin(event)

  const discordId = getRouterParam(event, 'discordId')

  if (!discordId) throw createError({ statusCode: 400, statusMessage: 'Missing discordId' })

  await signOutEverywhere(discordId)
  signOutHost(discordId, 'You were signed out. Sign in again.')

  await logAdminAction(adminId, 'sign-out-everywhere', discordId, null)

  return { success: true }
})
