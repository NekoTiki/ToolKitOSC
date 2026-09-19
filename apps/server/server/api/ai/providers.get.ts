import { getAiProviders } from '~~/server/utils/ai'

// Lets the client build its provider picker (and grey out/hide ones with no key configured on
// this server) without hardcoding the list or guessing at availability client-side.
export default defineEventHandler((event) => {
  verifyDesktopToken(event)

  return getAiProviders().map((provider) => ({
    id: provider.id,
    label: provider.label,
    configured: provider.isConfigured()
  }))
})
