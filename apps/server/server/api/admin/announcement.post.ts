import { z } from 'zod'

import { logAdminAction } from '~~/server/db/adminLog'
import { setNotice } from '~~/server/utils/serverNotice'

const bodySchema = z.object({
  text: z.string().trim().min(1).max(500),
  level: z.enum(['info', 'warning']),
  // Empty: shown until an admin clears it.
  minutes: z.number().int().min(1).max(7 * 24 * 60).optional(),
  onSharePages: z.boolean()
})

export default defineEventHandler(async (event) => {
  const { discordId: adminId } = await requireAdmin(event)

  const { text, level, minutes, onSharePages } = await readValidatedBody(event, bodySchema.parse)

  await logAdminAction(adminId, 'announce', null, { text, level, minutes, onSharePages })

  return { notice: setNotice({ text, level, onSharePages, expiresAt: minutes ? Date.now() + minutes * 60_000 : null }) }
})
