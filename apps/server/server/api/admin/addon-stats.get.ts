import { addonUsageTotals } from '~~/server/utils/addonStats'

// Which add-ons the accounts seen in the last 30 days have on, and their device counts.
export default defineEventHandler(async (event) => {
  await requireAdmin(event)

  return addonUsageTotals(30)
})
