import { randomUUID } from 'node:crypto'

interface Ticket {
  discordId: string
  expiresAt: number
}

const TICKET_TTL_MS = 30_000

const tickets = new Map<string, Ticket>()

function sweepExpired(): void {
  const now = Date.now()

  for (const [ticket, entry] of tickets) {
    if (entry.expiresAt <= now) tickets.delete(ticket)
  }
}

// Short-lived, single-use ticket authenticating a /admin/ws connection - the WS upgrade has no
// access to the admin's session cookie the way an ordinary REST call does (see
// server/routes/admin/ws.ts), so the browser first mints one of these over a normal
// requireAdmin-gated REST call (api/admin/ws-ticket.get.ts), then passes it as a query param when
// opening the socket.
export function mintAdminWsTicket(discordId: string): string {
  sweepExpired()

  const ticket = randomUUID()

  tickets.set(ticket, { discordId, expiresAt: Date.now() + TICKET_TTL_MS })

  return ticket
}

// Single-use: deletes the ticket whether or not it turns out to be valid, so it can't be replayed.
// Returns the discord id it was minted for, or null if the ticket is missing/expired.
export function consumeAdminWsTicket(ticket: string): string | null {
  const entry = tickets.get(ticket)

  tickets.delete(ticket)

  if (!entry || entry.expiresAt < Date.now()) return null

  return entry.discordId
}
