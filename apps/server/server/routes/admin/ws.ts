import type { Peer } from 'crossws'

import { consumeAdminWsTicket } from '~~/server/utils/ai/adminWsTickets'

const adminPeers = new Set<Peer>()

// Called whenever something the admin stats dashboard displays changes (see
// generationLog.ts's logGenerationAttempt) - pushes the actual changed data (e.g. the new
// generation log row), not just a "something changed" signal, so a connected dashboard can splice
// it straight into its existing state instead of re-running its REST fetches on every single
// event. A no-op with no dashboards currently connected.
export function notifyAdmins(message: unknown): void {
  const payload = JSON.stringify(message)

  for (const peer of adminPeers) peer.send(payload)
}

export default defineWebSocketHandler({
  // h3 bundles its own copy of crossws, nominally distinct from the root one `Peer` above is typed
  // against (same mismatch, same fix as routes/host.ts) - cast through unknown rather than typing
  // the hook params directly.
  open(_peer) {
    const peer: Peer = _peer as unknown as Peer

    // Authenticated via a short-lived single-use ticket (see adminWsTickets.ts), not a
    // cookie/session check here directly - the WS upgrade doesn't go through requireUserSession
    // the way a normal REST call does, so the browser mints one over an ordinary authenticated
    // REST call first (api/admin/ws-ticket.get.ts) and passes it as a query param to open this.
    const ticket = new URL(peer.request.url).searchParams.get('ticket')
    const discordId = ticket ? consumeAdminWsTicket(ticket) : null

    if (!discordId) {
      peer.close()
      return
    }

    adminPeers.add(peer)
  },
  close(_peer) {
    adminPeers.delete(_peer as unknown as Peer)
  },
  error(_peer, error) {
    console.error('[admin-ws] error:', error)
  }
})
