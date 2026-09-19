import { mintAdminWsTicket } from '~~/server/utils/ai/adminWsTickets'

// Mints a short-lived ticket the dashboard immediately trades in to open /admin/ws (see
// server/routes/admin/ws.ts) - the WS upgrade itself can't go through requireUserSession the way
// this route does, so this REST call is the actual auth check, once, up front.
export default defineEventHandler(async (event) => {
  const { discordId } = await requireAdmin(event)

  return { ticket: mintAdminWsTicket(discordId) }
})
