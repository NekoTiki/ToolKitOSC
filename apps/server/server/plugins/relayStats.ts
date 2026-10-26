import { ensureMigrated } from '~~/server/db'
import { flushRelayStats, loadRelayStats } from '~~/server/utils/relayStats'

export default defineNitroPlugin(async () => {
  await ensureMigrated()
  await loadRelayStats()

  setInterval(() => void flushRelayStats().catch((error) => console.error('[relay-stats] save failed:', error)), 60_000)
})
