import { resetMainShareCode } from '~~/server/db/shareLinks'

// A new random link. The old one, and the links built from the Discord id, 404 from now on.
export default defineEventHandler(async (event) => {
  const discordId = await shareLinkOwner(event)

  return { main: await resetMainShareCode(discordId) }
})
