import { searchUsers } from '~~/server/db/users'
import { searchUsersQuerySchema } from '~~/server/utils/ai/schemas'

// Powers the "add user" picker on /dashboard/users - searches known users (anyone who's ever
// connected) by id/username/display name, so an admin doesn't have to paste a raw Discord
// snowflake for someone who's already shown up once.
export default defineEventHandler(async (event) => {
  await requireAdmin(event)
  const { q } = await getValidatedQuery(event, searchUsersQuerySchema.parse)

  return { users: await searchUsers(q) }
})
