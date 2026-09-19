import { mkdirSync } from 'node:fs'
import { dirname } from 'node:path'

import { createClient } from '@libsql/client/node'
import { migrate } from 'drizzle-orm/libsql/migrator'
import { drizzle } from 'drizzle-orm/libsql/node'

import * as schema from './schema'

export type Db = ReturnType<typeof drizzle<typeof schema>>

// Lazy singleton, not a bare module-level const - useRuntimeConfig() is only guaranteed available
// once Nitro's own startup has run (plugins, request handlers), not necessarily at raw ESM import
// time, so the actual client is only ever created from inside a function call.
let dbInstance: Db | null = null

function createDb(): Db {
  const sqlitePath = useRuntimeConfig().sqlitePath

  // libsql (SQLITE_CANTOPEN / error 14) won't create a missing parent directory itself - the
  // production Docker image creates /app/data explicitly (see Dockerfile), but dev has nothing
  // else that would, so this covers both.
  mkdirSync(dirname(sqlitePath), { recursive: true })

  const client = createClient({ url: `file:${sqlitePath}` })

  return drizzle({ client, schema })
}

export function getDb(): Db {
  dbInstance ??= createDb()

  return dbInstance
}

// Runs once at server startup (see server/plugins/db.ts) - memoized so accidentally calling this
// more than once is harmless.
let migrated: Promise<void> | null = null

export function ensureMigrated(): Promise<void> {
  migrated ??= migrate(getDb(), { migrationsFolder: 'server/db/migrations' })

  return migrated
}
