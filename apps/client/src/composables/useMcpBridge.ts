import { useClientsDb } from '@renderer/composables/useClientsDb'
import { useControls } from '@renderer/composables/useControls'
import { useLockedControls } from '@renderer/composables/useLockedControls'
import { useViewerGroups } from '@renderer/composables/useViewerGroups'
import { db as clientsDb } from '@renderer/db/clients.db'
import { db as commandsDb } from '@renderer/db/commands.db'
import type { McpClient, McpRequest } from '@renderer/lib/tauri-bridge'
import { api } from '@renderer/lib/tauri-bridge'
import { getUUID } from '@renderer/utils/uuid'
import { ownsCommand, viewerLookup } from '@renderer/utils/viewers'
import type { ControlType, CreatedBy } from '@toolkitosc/shared-ui'
import { KNOWN_CONTROL_TYPES } from '@toolkitosc/shared-ui'
import type { Ref } from 'vue'
import { ref, toRaw } from 'vue'

// Rust's MCP server (src-tauri/src/mcp) asks here for what lives in the webview: controls,
// profiles, viewer groups, viewers and Activity, and the approval pop-up for tools on Ask. Every
// request gets an answer, or Rust gives up after a few seconds. Called once, from App.vue.

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
  const { controls, getControl, addGroup, updateGroup, setGroupHidden, addControl, updateControl, deleteControl } = useControls()
  const { lockedControlGroups, currentLockedControlsGroup, addLockedControlGroup, updateLockedControlGroup, removeLockedControlGroup } =
    useLockedControls()
  const { viewerGroups, memberCount, membershipsOf } = useViewerGroups()
  const { isOnline } = useClientsDb()

  // The same data the editors show, as plain JSON.
  const plain = <T>(value: T): T => JSON.parse(JSON.stringify(toRaw(value)))

  const findControl = (groupId: unknown, controlId: unknown): unknown => {
    const control = controls.value.find((group) => group.id === groupId)?.controls.find((control) => control.id === controlId)
    if (!control) throw new Error('No such control: get the ids from list_controls')

    return plain(control)
  }

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

  // Drafts: what an assistant makes carries its name until you keep it or edit it.
  const draftOf = (createdBy: CreatedBy | undefined, client: string): boolean => createdBy?.assistant === client

  // New controls go into a hidden group per assistant, so viewers never see a draft.
  const createControls = (args: Record<string, unknown>, client: string): unknown => {
    const list = Array.isArray(args.controls) ? args.controls : []
    const invalid = list.findIndex(
      (control) => typeof control?.name !== 'string' || !KNOWN_CONTROL_TYPES.includes(control?.type)
    )
    if (!list.length || invalid !== -1) throw new Error(`Control ${invalid + 1} needs a name and a type from tkosc://docs/control-types`)

    const name = `From ${client}`
    let groupId = controls.value.find((group) => group.name === name)?.id
    if (!groupId) {
      groupId = addGroup()
      updateGroup(groupId, name)
      setGroupHidden(groupId, true)
    }

    const createdBy = { assistant: client, at: Date.now() }
    const ids = list.map((control) => {
      const id = getUUID()
      addControl(groupId!, { ...control, id, createdBy } as ControlType)

      return id
    })

    return { groupId, controlIds: ids }
  }

  const updateOneControl = (args: Record<string, unknown>): unknown => {
    const control = getControl(String(args.groupId), String(args.controlId))
    if (!control) throw new Error('No such control: get the ids from list_controls')
    const changes = { ...(args.changes as Record<string, unknown>) }
    delete changes.id
    delete changes.type
    delete changes.createdBy
    updateControl(String(args.groupId), { ...control, ...changes } as ControlType)

    return null
  }

  const saveProfile = (args: Record<string, unknown>, client: string): unknown => {
    const data = {
      name: String(args.name),
      lockedControls: (args.lockedControls ?? {}) as Record<string, boolean>,
      limits: (args.limits ?? {}) as never,
      groupOverrides: (args.groupOverrides ?? {}) as never,
      createdBy: { assistant: client, at: Date.now() }
    }
    if (!args.id) return { id: addLockedControlGroup(data) }

    const existing = lockedControlGroups.value.find((profile) => profile.id === args.id)
    if (!existing) throw new Error('No such profile: leave id out to make a new one')
    if (!draftOf(existing.createdBy, client)) throw new Error("That one isn't your draft: the host made it, or kept it")
    updateLockedControlGroup(existing.id, data)

    return { id: existing.id }
  }

  const deleteDraft = (args: Record<string, unknown>, client: string): unknown => {
    if (args.kind === 'control') {
      const control = getControl(String(args.groupId), String(args.id))
      if (!control) throw new Error('No such control')
      if (!draftOf(control.createdBy, client)) throw new Error("That one isn't your draft")
      deleteControl(String(args.groupId), control.id)
    } else {
      const profile = lockedControlGroups.value.find((p) => p.id === args.id)
      if (!profile) throw new Error('No such profile')
      if (!draftOf(profile.createdBy, client)) throw new Error("That one isn't your draft")
      removeLockedControlGroup(profile.id)
    }

    return null
  }

  const handlers: Record<string, (args: Record<string, unknown>, request: McpRequest) => unknown> = {
    list_controls: () => plain(controls.value),
    get_control: (args) => findControl(args.groupId, args.controlId),
    list_profiles: () => ({ active: currentLockedControlsGroup.value, profiles: plain(lockedControlGroups.value) }),
    list_viewer_groups: () => viewerGroups.value.map((group) => ({ id: group.id, name: group.name, members: memberCount(group.id) })),
    list_viewers: () => viewers(),
    switch_profile: (args) => {
      if (args.id !== null && !lockedControlGroups.value.some((profile) => profile.id === args.id)) throw new Error('No such profile: get the ids from list_profiles')
      currentLockedControlsGroup.value = (args.id as string | null) ?? null

      return null
    },
    get_activity: (args) => activity(args),
    create_controls: (args, request) => createControls(args, request.client),
    update_control: (args) => updateOneControl(args),
    save_profile: (args, request) => saveProfile(args, request.client),
    delete_draft: (args, request) => deleteDraft(args, request.client),
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
