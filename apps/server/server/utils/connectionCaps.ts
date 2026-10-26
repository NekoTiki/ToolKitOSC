// How many share-page connections one IP and one room may have. The rate limit is per session, and
// a script makes a new guest session per connection, so these are what bound a flood.
export const MAX_SOCKETS_PER_IP = 10
export const MAX_VIEWERS_PER_ROOM = 200

export type ConnectionRefusal = 'too-many-connections' | 'room-full'

interface Counts {
  // Open share-page sockets from this IP, in every room, before this one.
  socketsFromIp: number
  // Viewers already in the room.
  viewersInRoom: number
  // Another tab of a viewer who's already in the room: one more socket, not one more viewer.
  alreadyInRoom: boolean
}

export function connectionRefusal(counts: Counts, limits = { perIp: MAX_SOCKETS_PER_IP, perRoom: MAX_VIEWERS_PER_ROOM }): ConnectionRefusal | null {
  if (counts.socketsFromIp >= limits.perIp) return 'too-many-connections'
  if (!counts.alreadyInRoom && counts.viewersInRoom >= limits.perRoom) return 'room-full'

  return null
}
