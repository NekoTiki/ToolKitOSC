import { accessRequestStatus } from '~~/server/utils/ai/access'

// Where this account's request for AI suggestions stands, for the app's AI page.
export default defineEventHandler(async (event) => {
  const user = await verifyDesktopToken(event)

  if (!user.discord?.id) throw createError({ statusCode: 401, statusMessage: 'No Discord identity on token' })

  return { status: await accessRequestStatus(user.discord.id) }
})
