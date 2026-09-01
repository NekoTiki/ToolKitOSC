import type { Peer } from 'crossws'

type PeerId = string

export const peers = new Map<PeerId, Peer>()

export const sendTokenToPeer = (peerId: PeerId, token: string): boolean => {
  const peer = peers.get(peerId)

  if (!peer) return false

  peer.send({
    type: 'auth-token',
    message: { token }
  })

  return true
}

export default defineWebSocketHandler({
  async open(_peer) {
    const peer: Peer = _peer as unknown as Peer

    peers.set(peer.id, peer)

    const dev = import.meta.dev
    const host = peer.request.headers.get('host') || 'unknown host'
    const protocol = dev ? 'http' : 'https'

    peer.send({
      type: 'auth-uri',
      message: {
        peerId: peer.id,
        authUrl: `${protocol}://${host}/auth/desktop/discord?peerId=${peer.id}`
      }
    })
  },
  message(peer, message) {
    if (message.text().includes('ping')) {
      peer.send('pong')
      return
    }
  },
  close(peer) {
    peers.delete(peer.id)
  },
  error(peer, error) {
    console.error('[ws] error', peer, error)
  }
})
