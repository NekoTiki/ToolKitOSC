import { ensureMigrated } from '~~/server/db'

// Runs pending drizzle migrations (server/db/migrations/) once, before the server starts serving
// requests - Nitro awaits plugin callbacks during startup, so any route handler that touches the
// db only ever runs after this resolves.
export default defineNitroPlugin(async () => {
  await ensureMigrated()
})
