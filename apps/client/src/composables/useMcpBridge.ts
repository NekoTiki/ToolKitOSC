import { useClientsDb } from '@renderer/composables/useClientsDb'
import { useViewerGroups } from '@renderer/composables/useViewerGroups'
import { db as clientsDb } from '@renderer/db/clients.db'
import { db as commandsDb } from '@renderer/db/commands.db'
import type { McpClient, McpRequest } from '@renderer/lib/tauri-bridge'
import { api } from '@renderer/lib/tauri-bridge'
import { ownsCommand, viewerLookup } from '@renderer/utils/viewers'
import type { Ref } from 'vue'
import { ref } from 'vue'

// Rust's MCP server (src-tauri/src/mcp) asks here for what lives in the webview: viewers and
// Activity, and the approval pop-up for tools on Ask. Controls, profiles and viewer groups it
// answers itself. Every request gets an answer, or Rust gives up after a few seconds. Called once,
// from App.vue.

export type ApprovalAnswer = 'allow' | 'allow-10-min' | 'deny'

export interface Approval {
  id: number
  client: string
  tool: string
  arguments: Record<string, unknown> | null
}

// Waiting for you, oldest first. ApprovalPopup.vue shows the first.
const approvals = ref<Approval[]>([])

const answerApproval = (id: number, answer: ApprovalAnswer): void => {
  approvals.value = approvals.value.filter((approval) => approval.id !== id)
  void api.mcpRespond(id, { result: answer })
}

export function useMcpApprovals(): { approvals: Ref<Approval[]>; answerApproval: typeof answerApproval } {
  return { approvals, answerApproval }
}

// The assistants connected now, for Settings > Assistants and the Viewers page.
const clients = ref<McpClient[]>([])

api.onMcpClients((value) => (clients.value = value))
void api.mcpClients().then((value) => (clients.value = value))

export function useMcpClients(): Ref<McpClient[]> {
  return clients
}

const MAX_ACTIVITY = 500

export function useMcpBridge(): void {
  const { membershipsOf } = useViewerGroups()
  const { isOnline } = useClientsDb()

  const viewers = async (): Promise<unknown[]> =>
    (await clientsDb.clients.toArray()).map((client) => ({
      untrustedName: client.displayName,
      online: isOnline(client),
      signedIn: !!client.discordId,
      groups: membershipsOf(client).map((membership) => membership.group.name),
      lastSeenAt: client.lastSeenAt ? new Date(client.lastSeenAt).toISOString() : null
    }))

  const activity = async (args: Record<string, unknown>): Promise<unknown[]> => {
    const limit = Math.min(Number(args.limit) || 100, MAX_ACTIVITY)
    const from = args.minutes ? Date.now() - Number(args.minutes) * 60_000 : 0
    const clients = await clientsDb.clients.toArray()
    const viewerOf = viewerLookup(clients)
    const wanted = args.viewer ? clients.filter((client) => client.displayName === args.viewer) : null

    const rows = await commandsDb.commands
      .where('createdAt')
      .aboveOrEqual(from)
      .reverse()
      .filter((command) => (!args.controlId || command.controlId === args.controlId) && (!wanted || wanted.some((client) => ownsCommand(client, command))))
      .limit(limit)
      .toArray()

    return rows.map((command) => ({
      at: new Date(command.createdAt).toISOString(),
      untrustedViewer: viewerOf(command)?.displayName ?? 'Guest',
      groupId: command.groupId,
      controlId: command.controlId,
      control: command.controlName,
      type: command.type,
      value: command.value ?? null
    }))
  }

  const handlers: Record<string, (args: Record<string, unknown>, request: McpRequest) => unknown> = {
    list_viewers: () => viewers(),
    get_activity: (args) => activity(args),
    approve: (args, request) => {
      approvals.value = [
        ...approvals.value,
        { id: request.id, client: request.client, tool: String(args.tool), arguments: (args.arguments as Record<string, unknown> | null) ?? null }
      ]

      // Rust counts it as Deny after a minute: the pop-up goes too.
      setTimeout(() => (approvals.value = approvals.value.filter((approval) => approval.id !== request.id)), 60_000)

      // Answered from the pop-up instead.
      return undefined
    }
  }

  api.onMcpRequest(async (request) => {
    const handler = handlers[request.kind]
    try {
      if (!handler) throw new Error(`ToolKitOSC can't answer ${request.kind}`)
      const result = await handler(request.args, request)
      if (request.kind !== 'approve') await api.mcpRespond(request.id, { result })
    } catch (error) {
      await api.mcpRespond(request.id, { error: error instanceof Error ? error.message : String(error) })
    }
  })
}
