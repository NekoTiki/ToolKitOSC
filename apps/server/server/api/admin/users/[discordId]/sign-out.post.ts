import { signOutEverywhere } from '~~/server/db/accounts'
import { signOutHost } from '~~/server/routes/host'

// Every desktop token made before now stops working (a leaked token, a shared PC).
export default defineEventHandler(async (event) => {
  await requireAdmin(event)

  const discordId = getRouterParam(event, 'discordId')

  if (!discordId) throw createError({ statusCode: 400, statusMessage: 'Missing discordId' })

  await signOutEverywhere(discordId)
  signOutHost(discordId, 'You were signed out. Sign in again.')

  return { success: true }
})
