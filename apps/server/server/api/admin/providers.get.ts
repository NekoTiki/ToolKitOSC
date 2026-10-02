import { getAiProviders } from '~~/server/utils/ai'

// The configured AI providers, for the provider filter on /dashboard/generations.
export default defineEventHandler(async (event): Promise<{ providers: { id: string; label: string }[] }> => {
  await requireAdmin(event)

  return {
    providers: getAiProviders()
      .filter((provider) => provider.isConfigured())
      .map(({ id, label }) => ({ id, label }))
  }
})
