import { sendMessageToHost } from '~~/server/routes/host'
import { setCanSelectModel } from '~~/server/utils/ai/access'
import { setCanSelectModelBodySchema } from '~~/server/utils/ai/schemas'

export default defineEventHandler(async (event) => {
  await requireAdmin(event)

  const discordId = getRouterParam(event, 'discordId')

  if (!discordId) throw createError({ statusCode: 400, statusMessage: 'Missing discordId' })

  const { canSelectModel } = await readValidatedBody(event, setCanSelectModelBodySchema.parse)

  await setCanSelectModel(discordId, canSelectModel)
  sendMessageToHost(discordId, 'ai-access-update', { hasAccess: true, canSelectModel })

  return { success: true }
})
