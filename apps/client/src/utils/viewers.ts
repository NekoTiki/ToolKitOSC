import type { Client } from '@renderer/db/clients.db'
import type { Command } from '@renderer/db/commands.db'
import { db } from '@renderer/db/commands.db'
import type { Collection } from 'dexie'

// The one rule for "whose command is this", used by the Viewers and Activity pages and the
// Controls page's last-user badges. A Discord account owns its commands plus the ones it sent as a
// guest before logging in; a guest owns their session's; a viewer recorded before sessions were
// tracked owns the guest commands from their IP.
const isLegacy = (client: Client): boolean => client.key.startsWith('legacy-ip:')

export const ownsCommand = (client: Client, command: Command): boolean => {
  if (command.discordId) return command.discordId === client.discordId
  if (isLegacy(client)) return command.ip === client.ip

  return client.peerIds.includes(command.peerId)
}

// Indexed queries that together cover a viewer's commands since `from`, newest first. They never
// overlap, so counts can be added up.
const viewerCollections = (client: Client, from: number): Collection<Command, number>[] => {
  const guestCommands = (command: Command): boolean => !command.discordId

  if (isLegacy(client)) {
    return [db.commands.where('[ip+createdAt]').between([client.ip, from], [client.ip, Infinity]).reverse().filter(guestCommands)]
  }

  return [
    ...(client.discordId ? [db.commands.where('[discordId+createdAt]').between([client.discordId, from], [client.discordId, Infinity]).reverse()] : []),
    ...client.peerIds.map((peerId) =>
      db.commands.where('[peerId+createdAt]').between([peerId, from], [peerId, Infinity]).reverse().filter(guestCommands)
    )
  ]
}

const newestFirst = (lists: Command[][], limit: number): Command[] =>
  lists
    .flat()
    .sort((a, b) => b.createdAt - a.createdAt)
    .slice(0, limit)

export const viewerCommands = (
  client: Client,
  { from = 0, limit = Infinity, filter }: { from?: number; limit?: number; filter?: (command: Command) => boolean } = {}
): Promise<Command[]> =>
  Promise.all(
    viewerCollections(client, from).map((collection) => (filter ? collection.filter(filter) : collection).limit(limit).toArray())
  ).then((lists) => newestFirst(lists, limit))

export const countViewerCommands = (client: Client): Promise<number> =>
  Promise.all(viewerCollections(client, 0).map((collection) => collection.count())).then((counts) => counts.reduce((a, b) => a + b, 0))

// Finds the viewer a logged command belongs to, without scanning every viewer per row.
export const viewerLookup = (clients: Client[]): ((command: Command) => Client | undefined) => {
  const byDiscordId = new Map<string, Client>()
  const byPeerId = new Map<string, Client>()
  const byLegacyIp = new Map<string, Client>()

  clients.forEach((client) => {
    if (client.discordId) byDiscordId.set(client.discordId, client)
    if (isLegacy(client)) byLegacyIp.set(client.ip, client)
    client.peerIds.forEach((peerId) => byPeerId.set(peerId, client))
  })

  return (command) =>
    command.discordId ? byDiscordId.get(command.discordId) : (byPeerId.get(command.peerId) ?? byLegacyIp.get(command.ip))
}
