import { deleteAccountData, isSuspended } from '~~/server/db/accounts'
import { signOutHost } from '~~/server/routes/host'

// "Delete my data", from the app (bearer token) or the website (session). A suspended account
// can't: that would clear the suspension too.
export default defineEventHandler(async (event) => {
  const user = getHeader(event, 'authorization') ? await verifyDesktopToken(event) : (await requireUserSession(event)).user
  const discordId = user.discord?.id

  if (!discordId) throw createError({ statusCode: 401, statusMessage: 'No Discord identity' })
  if (await isSuspended(discordId)) throw createError({ statusCode: 403, statusMessage: 'A suspended account can’t delete its data. Ask an admin of this server.' })

  await deleteAccountData(discordId)
  signOutHost(discordId, 'Your data was deleted from this server.')

  return { success: true }
})
