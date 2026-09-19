import { sendMessageToHost } from '~~/server/routes/host'
import { grantAccess } from '~~/server/utils/ai/access'
import { grantAccessBodySchema } from '~~/server/utils/ai/schemas'

export default defineEventHandler(async (event) => {
  const { discordId: grantedBy } = await requireAdmin(event)
  const body = await readValidatedBody(event, grantAccessBodySchema.parse)

  await grantAccess(body.discordId, grantedBy, body.canSelectModel, body.note)

  // No-op if that account has no live host WS connection right now - it'll see its access next
  // time it connects either way (auth-success carries the current state, see routes/host.ts).
  sendMessageToHost(body.discordId, 'ai-access-update', { hasAccess: true, canSelectModel: body.canSelectModel })

  return { success: true }
})
