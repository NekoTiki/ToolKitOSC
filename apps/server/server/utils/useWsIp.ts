import type { Peer } from 'crossws'

export const useWsIp = (peer: Peer) => {
  const xForwardedFor = peer.request.headers.get('x-forwarded-for')
  const wsAddr = peer.remoteAddress

  if (xForwardedFor !== 'undefined' && xForwardedFor) return xForwardedFor
  if (!wsAddr) return 'unknown'

  return wsAddr.startsWith('::ffff:') ? wsAddr.replace('::ffff:', '') : wsAddr
}
