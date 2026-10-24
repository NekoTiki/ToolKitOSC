import { appVersionCounts } from '~~/server/db/users'

// Accounts seen in the last 30 days, by app version and OS: when nobody runs an old version any
// more, the minimum supported protocol can go up.
export default defineEventHandler(async (event) => {
  await requireAdmin(event)

  return appVersionCounts(30)
})
