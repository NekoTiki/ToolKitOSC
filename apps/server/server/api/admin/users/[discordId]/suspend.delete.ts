import { unsuspend } from '~~/server/db/accounts'

export default defineEventHandler(async (event) => {
  await requireAdmin(event)

  const discordId = getRouterParam(event, 'discordId')

  if (!discordId) throw createError({ statusCode: 400, statusMessage: 'Missing discordId' })

  await unsuspend(discordId)

  return { success: true }
})
