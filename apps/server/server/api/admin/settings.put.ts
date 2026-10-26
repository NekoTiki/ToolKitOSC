import { logAdminAction } from '~~/server/db/adminLog'
import { getSettings, saveSettings, settingsSchema } from '~~/server/utils/serverSettings'

export default defineEventHandler(async (event) => {
  const { discordId: adminId } = await requireAdmin(event)
  const changes = await readValidatedBody(event, settingsSchema.partial().parse)
  const before = getSettings()
  const changed = Object.fromEntries(Object.entries(changes).filter(([key, value]) => before[key as keyof typeof before] !== value))

  const settings = await saveSettings(changed)

  if (Object.keys(changed).length) await logAdminAction(adminId, 'change-settings', null, changed)

  return { settings }
})
