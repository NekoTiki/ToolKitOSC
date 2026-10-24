import { useAiAccess } from '@renderer/composables/useAiAccess'
import { useAiCredits } from '@renderer/composables/useAiCredits'
import { useAiGenerationStatus } from '@renderer/composables/useAiGenerationStatus'
import { useAuth } from '@renderer/composables/useAuth'
import { useChatbox } from '@renderer/composables/useChatbox'
import { describeViewer, useClientsDb } from '@renderer/composables/useClientsDb'
import { useClientType } from '@renderer/composables/useClientType'
import { useCommandsDb } from '@renderer/composables/useCommandsDb'
import { useControls } from '@renderer/composables/useControls'
import { onPatternRunEnd } from '@renderer/composables/useIntifacePatterns'
import { useShareSettings } from '@renderer/composables/useShareSettings'
import { useTheme } from '@renderer/composables/useTheme'
import { serverWsUrl } from '@renderer/composables/useWebsocketSettings'
import type { ServerPolicy, ServerStatus } from '@renderer/lib/tauri-bridge'
import { api } from '@renderer/lib/tauri-bridge'
import { assistantName } from '@renderer/utils/viewers'
import type { ClientListEntry, ControlTypes, RoomStatsMessage, ServerNotice, WaitingViewer } from '@toolkitosc/shared-ui'
import { PROTOCOL_VERSION, useBhapticsControl, useIntifaceControl, useIntifacePatternControl, useOpenShockControl, useOwoControl } from '@toolkitosc/shared-ui'
import type { WebSocketStatus } from '@vueuse/core'
import type { ComputedRef } from 'vue'
import { watch } from 'vue'
import { computed, ref } from 'vue'

export type Client = ClientListEntry

export const clients = ref<Client[]>([])

// `clients` has one entry per connection, so the same person on several browsers/tabs shows up
// more than once - this counts people instead, keyed the same way the Viewers page groups them.
// Signed-in viewers waiting on the share page for you to let them in ("Only people you let in").
export const waitingViewers = ref<WaitingViewer[]>([])

// This stream so far, as the server counts it: when it started and the most viewers at once.
export const roomStats = ref<RoomStatsMessage | null>(null)

// Letting one in is adding them to the approved list (useShareSettings.ts sends it).
export const declineWaitingViewer = (peerId: string): void => void api.serverSend(JSON.stringify({ type: 'viewer-declined', message: { peerId } }))

export const uniqueClientCount = computed(() => new Set(clients.value.map((client) => describeViewer(client).key)).size)

// The socket itself lives in Rust (src-tauri/src/server), so it stays up while this window reloads
// or sits in the tray. Rust only keeps one, but only StatusBar.vue calls useWebsocketHost(), since
// the message handler in it must run once. Other places - e.g. Settings → Account's server status
// line - read the connection state from here.
export const hostStatus = ref<WebSocketStatus>('CLOSED')
const serverStatus = ref<ServerStatus>('closed')
// Set by useWebsocketHost(). The listeners are registered at module scope, and connecting waits
// for them, so nothing Rust replays on connect is missed (see useAvatarDetails.ts).
let onServerText: ((text: string) => void) | undefined
let onServerDebug: ((text: string) => void) | undefined
const listening = Promise.all([
  api.onServerStatus((status) => (serverStatus.value = status)),
  api.onServerMessage((text) => onServerText?.(text)),
  api.onServerDebug((text) => onServerDebug?.(text))
])
// Set from auth-success/protocol-mismatch (both report the server's own version, whether or not
// it accepted this client) so the "Server" tooltip can show a client/server version pair even
// though a real mismatch closes the connection before `hostStatus` ever reaches 'OPEN'.
export const serverProtocolVersion = ref<number | null>(null)
export const protocolMismatchReason = ref<string | null>(null)
// Every control type the connected server can currently render (from auth-success) - null until
// then, since we don't know yet.
export const serverSupportedControlTypes = ref<ControlTypes[] | null>(null)
// Control types actually sent to the server (i.e. from `visibleControls`, same set `sendControls`
// forwards - a hidden group's controls never reach the server at all, so their types can't be
// unsupported in any way that matters) that `serverSupportedControlTypes` doesn't list - controls
// this client can create but the connected server can't display for viewers yet. Recomputed by
// the watch in useWebsocketHost() below.
export const unsupportedControlTypes = ref<ControlTypes[]>([])
// The server sends its notice again on every reconnect: show each one once.
let shownNoticeId: string | null = null

const showServerNotice = (notice: ServerNotice | null): void => {
  if (!notice || notice.id === shownNoticeId) return

  shownNoticeId = notice.id
  useToast().add({
    title: 'Message from the server',
    description: notice.text,
    icon: 'i-lucide-megaphone',
    color: notice.level === 'warning' ? 'warning' : 'info',
    duration: 20_000
  })
}

const clientsMap = computed<Record<string, Client>>(() => {
  const map: Record<string, Client> = {}

  clients.value.forEach((client) => (map[client.peerId] = client))

  return map
})

export function useWebsocketHost(): {
  clients: typeof clients
  status: ComputedRef<WebSocketStatus>
  sendMessage: (type: string, message: unknown) => void
  open: () => void
  close: () => void
} {
  // Rust sends the controls viewers may see, the parameters they use and who is in which viewer
  // group to the server itself (src-tauri/src/setup/sync.rs).
  const { visibleControls, setControlLastUser, getControl } = useControls()
  const { user } = useAuth()

  watch(
    [visibleControls, serverSupportedControlTypes],
    () => {
      if (!serverSupportedControlTypes.value) {
        unsupportedControlTypes.value = []
        return
      }

      const supported = new Set(serverSupportedControlTypes.value)
      const found = new Set<ControlTypes>()

      visibleControls.value?.forEach((group) => {
        group.controls.forEach((control) => {
          if (!supported.has(control.type)) found.add(control.type)
        })
      })

      unsupportedControlTypes.value = Array.from(found)
    },
    { deep: true, immediate: true }
  )

  const { onlineClients, upsertViewers } = useClientsDb()
  const { add: addCommandToDb, update: updateCommandInDb } = useCommandsDb()
  const { lastUpdate } = useOpenShockControl()
  const { lastUpdate: intifaceLastUpdate } = useIntifaceControl()
  const { lastUpdate: intifacePatternLastUpdate } = useIntifacePatternControl()
  const { state: bhapticsState } = useBhapticsControl()
  const { state: owoState } = useOwoControl()
  const { clientType } = useClientType()
  const { shareSettings, showViewers } = useShareSettings()
  const { announceAction } = useChatbox()
  const { selectedPrimary, selectedSecondary } = useTheme()

  // A viewer's command Rust checked and ran (src-tauri/src/server/commands.rs), or accepted
  // without a control to run: log it, note who used the control last, and post it to VRChat's
  // chatbox if the host turned that on. Rust already replied to the viewer and started automations.
  api.onCommandExecuted((executed) => {
    const client = clientsMap.value[executed.peerId]
    // An AI assistant's command (src-tauri/src/mcp) is logged and announced under its name.
    const assistant = assistantName(executed.peerId)

    if (!client && assistant === null) return

    const viewer = client ? describeViewer(client) : { key: executed.peerId, discordId: null, ip: '', displayName: assistant!, avatar: '' }
    const control = getControl(executed.groupId, executed.controlId)
    const accepted = {
      groupId: executed.groupId,
      controlId: executed.controlId,
      controlName: executed.controlName,
      type: executed.type as ControlTypes,
      value: executed.value
    }

    if (control) {
      setControlLastUser(executed.groupId, executed.controlId, {
        avatar: viewer.avatar,
        displayName: viewer.displayName,
        discordId: viewer.discordId ?? undefined,
        ip: viewer.ip
      })
    }

    // A viewer's own pattern is logged with its samples, and its row gets the time it stopped.
    const logged = addCommandToDb({ ...accepted, samples: executed.samples, discordId: viewer.discordId ?? undefined, peerId: executed.peerId, ip: viewer.ip })

    if (executed.patternRun !== undefined && executed.patternRun !== null) {
      onPatternRunEnd(executed.patternRun, () => {
        const endedAt = Date.now()

        void logged.then((id) => id !== undefined && updateCommandInDb(id, { endedAt }))
      })
    }

    // Names follow the share page's own setting. An assistant is always named.
    announceAction({ ...accepted, viewerKey: viewer.key, viewerName: showViewers.value || assistant !== null ? viewer.displayName : null, control })
  })

  // The share settings Rust checks viewer commands against: who may use controls at all, and
  // whether viewers see each other's names. Controls, groups and bans are Rust's own.
  const policy = computed<ServerPolicy>(() => ({
    clientType: clientType.value,
    showViewers: showViewers.value
  }))

  watch(policy, (value) => void api.serverSetPolicy(JSON.parse(JSON.stringify(value))), { deep: true, immediate: true })

  const status = computed<WebSocketStatus>(() =>
    serverStatus.value === 'open' ? 'OPEN' : serverStatus.value === 'connecting' ? 'CONNECTING' : 'CLOSED'
  )
  // From StatusBar's mount to its unmount. Rust retries a dropped link by itself, signs in with the
  // token it keeps for that server, and connects again when that token changes. A new server URL
  // connects again from here.
  let active = false

  const connect = async (): Promise<void> => {
    await listening
    if (active) await api.serverConnect(serverWsUrl.value, PROTOCOL_VERSION)
  }

  const open = (): void => {
    active = true
    void connect()
  }

  const close = (): void => {
    active = false
    void api.serverDisconnect()
  }

  watch(serverWsUrl, () => void connect())

  onServerDebug = (text) => logMessage('⇧', JSON.parse(text))

  onServerText = (text) => {
    const data = JSON.parse(text)

    logMessage('⇩', data)

    if (data.type === 'auth-success') {
      serverProtocolVersion.value = data.message.serverVersion
      serverSupportedControlTypes.value = data.message.supportedControlTypes
      protocolMismatchReason.value = null
      useAiAccess().applyUpdate(data.message.aiAccess)
      sendTheme()
      sendMessage('share-settings', shareSettings.value)
    } else if (data.type === 'auth-error') {
      // Rust forgets this token and connects again, signed out. The reason is the server's own
      // words: an expired token, a suspension, or an admin signing you out.
      console.error('Authentication failed:', data.message)
      useToast().add({ title: 'Signed out of the server', description: data.message, icon: 'i-lucide-log-out', color: 'warning', duration: 0 })
    } else if (data.type === 'protocol-mismatch') {
      console.error('Protocol mismatch:', data.message.reason)
      serverProtocolVersion.value = data.message.serverVersion
      protocolMismatchReason.value = data.message.reason
      useToast().add({
        title: 'App update required',
        description: data.message.reason,
        color: 'error'
      })
      // Not an auth problem - don't clear the token. Rust stops retrying against a server it can
      // never satisfy until the app is updated.
    } else if (data.type === 'update-username') {
      const client = clientsMap.value[data.from]

      if (client) {
        if (!client.user) {
          client.user = {
            userDefinedDisplayName: data.message.displayName
          }
        } else client.user.userDefinedDisplayName = data.message.displayName

        void upsertViewers([describeViewer(client)])
      }
    } else if (data.type === 'client-list') {
      clients.value = data.message

      const viewers = (data.message as Client[]).map(describeViewer)

      onlineClients.value = new Set(viewers.map((viewer) => viewer.key))
      void upsertViewers(viewers)
    } else if (data.type === 'ai-credits-update') {
      useAiCredits().applyUpdate(data.message)
    } else if (data.type === 'ai-access-update') {
      useAiAccess().applyUpdate(data.message)
    } else if (data.type === 'ai-generate-progress') {
      useAiGenerationStatus().applyProgress(data.message)
    } else if (data.type === 'ai-generate-result') {
      useAiGenerationStatus().applyResult(data.message)
    } else if (data.type === 'ai-generate-error') {
      useAiGenerationStatus().applyError(data.message)
    } else if (data.type === 'room-stats') {
      roomStats.value = data.message
    } else if (data.type === 'viewers-waiting') {
      waitingViewers.value = data.message
    } else if (data.type === 'server-notice') {
      showServerNotice(data.message)
    } else if (data.type === 'admin-stop') {
      // Rust already stopped everything.
      useToast().add({
        title: 'An admin of the server stopped everything',
        description: 'Resume when you are ready.',
        icon: 'i-lucide-octagon-x',
        color: 'warning'
      })
    }
  }

  watch(
    status,
    (value) => {
      hostStatus.value = value
      // Nobody can reach your controls while this socket is down, so nobody is online - this also
      // stamps everyone's "last seen" (see useClientsDb.ts). The server resends the list on
      // reconnect.
      if (value !== 'OPEN') onlineClients.value = new Set()
    },
    { immediate: true }
  )

  const logMessage = (arrow: '⇩' | '⇧', data: { type: string; message: unknown }): void => {
    console.log(
      `%c[${arrow} WS]%c[${user.value?.discord.id}]%c[${data.type}] %c${JSON.stringify(data.message)}`,
      'color: #42b983; font-weight: bold;',
      'color: #ba43ba; font-weight: bold;',
      'color: #437cba; font-weight: bold;',
      'color: #ffffff;'
    )
  }

  const sendMessage = (type: string, message: unknown): void => {
    logMessage('⇧', { type, message })
    void api.serverSend(JSON.stringify({ type, message }))
  }

  const sendTheme = (): void => {
    sendMessage('theme-update', {
      primary: selectedPrimary.value,
      secondary: selectedSecondary.value
    })
  }

  // These three are synchronous, so values set together (e.g. Stop everything zeroing every toy)
  // each reach viewers instead of only the last one.
  watch(
    () => lastUpdate.value,
    (newUpdate) => {
      if (!newUpdate) return

      sendMessage('open-shock-value-update', newUpdate)
    },
    { flush: 'sync' }
  )

  watch(
    () => intifaceLastUpdate.value,
    (newUpdate) => {
      if (!newUpdate) return

      sendMessage('intiface-value-update', newUpdate)
    },
    { flush: 'sync' }
  )

  watch(
    () => intifacePatternLastUpdate.value,
    (newUpdate) => {
      if (!newUpdate) return

      sendMessage('intiface-pattern-value-update', newUpdate)
    },
    { flush: 'sync' }
  )

  // Rust sends it only when it changes, at most every 100 ms (see bhaptics/mod.rs).
  watch(bhapticsState, (state) => sendMessage('bhaptics-value-update', state))
  // The same for the OWO Skin (see owo/mod.rs).
  watch(owoState, (state) => sendMessage('owo-value-update', state))

  watch([selectedPrimary, selectedSecondary], () => sendTheme())

  watch(shareSettings, (settings) => {
    if (status.value === 'OPEN') sendMessage('share-settings', settings)
  })

  return { clients, status, sendMessage, open, close }
}
