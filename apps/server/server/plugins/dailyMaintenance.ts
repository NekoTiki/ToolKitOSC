import { lt } from 'drizzle-orm'

import { ensureMigrated, getDb } from '~~/server/db'
import { aiCreditBonus, aiCreditUsage, aiGenerationLog, controlActivationDaily } from '~~/server/db/schema'

const RETENTION_DAYS = 90
const DAY_MS = 24 * 60 * 60 * 1000

// Credit counters are day-partitioned (see rateLimit.ts), so they already reset themselves the
// instant a new UTC day starts - no explicit reset write needed. This job's actual purpose is
// retention: prune rows old enough that nothing queries them anymore (the admin stats dashboard
// only ever looks back ~90 days), so the sqlite file doesn't grow forever.
async function pruneOldRows(): Promise<void> {
  const db = getDb()
  const cutoffDate = new Date(Date.now() - RETENTION_DAYS * DAY_MS)
  const cutoffDay = cutoffDate.toISOString().slice(0, 10)

  await Promise.all([
    db.delete(aiCreditUsage).where(lt(aiCreditUsage.day, cutoffDay)),
    db.delete(aiCreditBonus).where(lt(aiCreditBonus.day, cutoffDay)),
    db.delete(aiGenerationLog).where(lt(aiGenerationLog.createdAt, cutoffDate)),
    db.delete(controlActivationDaily).where(lt(controlActivationDaily.day, cutoffDay))
  ])
}

function runPrune(): void {
  void pruneOldRows().catch((error) => console.error('[ai] daily maintenance prune failed:', error))
}

export default defineNitroPlugin(async () => {
  // ensureMigrated() is memoized - calling it here too (on top of plugins/db.ts) just makes this
  // plugin's own db access safe regardless of Nitro's plugin load order, without relying on
  // filename ordering between the two.
  await ensureMigrated()

  runPrune()
  setInterval(runPrune, DAY_MS)
})
