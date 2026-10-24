import { customShareName, mainShareCode } from '~~/server/db/shareLinks'

// The host's own share link codes, for the app's Settings > Sharing and the connect page.
export default defineEventHandler(async (event) => {
  const discordId = await shareLinkOwner(event)

  return { main: await mainShareCode(discordId), custom: await customShareName(discordId) }
})
