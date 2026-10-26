import { sql } from 'drizzle-orm'

import { getDb } from '~~/server/db'
import { hostList } from '~~/server/routes/host'
import { clientList } from '~~/server/routes/ws/[ws]'

// For the Docker HEALTHCHECK and uptime monitors: 503 when the database doesn't answer.
export default defineEventHandler(async (event) => {
  let database = true

  try {
    await getDb().run(sql`select 1`)
  } catch (error) {
    console.error('[health] database check failed:', error)
    database = false
  }

  if (!database) setResponseStatus(event, 503)

  return {
    ok: database,
    database,
    uptimeSeconds: Math.round(process.uptime()),
    hosts: hostList.size,
    viewers: [...clientList.values()].reduce((sum, room) => sum + room.size, 0),
    memoryMb: Math.round(process.memoryUsage().rss / 1024 / 1024)
  }
})
