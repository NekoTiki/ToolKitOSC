import { ensureMigrated } from '~~/server/db'
import { loadSettings } from '~~/server/utils/serverSettings'

export default defineNitroPlugin(async () => {
  await ensureMigrated()
  await loadSettings()
})
