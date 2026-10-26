import { getAiProviders } from '~~/server/utils/ai'
import { getSettings } from '~~/server/utils/serverSettings'

export default defineEventHandler(async (event) => {
  await requireAdmin(event)

  return {
    settings: getSettings(),
    providers: getAiProviders()
      .filter((provider) => provider.isConfigured())
      .map(({ id, label }) => ({ id, label }))
  }
})
