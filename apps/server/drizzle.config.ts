import { defineConfig } from 'drizzle-kit'

// Only used by `npm run db:generate` (drizzle-kit generate) to diff server/db/schema.ts against
// the committed migration history in server/db/migrations/ - drizzle-kit doesn't need a live DB
// connection for that, so no `dbCredentials`/env var is required here.
export default defineConfig({
  dialect: 'sqlite',
  schema: './server/db/schema.ts',
  out: './server/db/migrations'
})
