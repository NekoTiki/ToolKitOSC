import { useAiAccess } from '@renderer/composables/useAiAccess'
import { useAiCredits } from '@renderer/composables/useAiCredits'
import { useAiGenerationStatus } from '@renderer/composables/useAiGenerationStatus'
import { useAuth } from '@renderer/composables/useAuth'
import { useAvatarDetails } from '@renderer/composables/useAvatarDetails'
import { useBannedClientsDb } from '@renderer/composables/useBannedClientsDb'
import { useChatbox } from '@renderer/composables/useChatbox'
import { describeViewer, useClientsDb } from '@renderer/composables/useClientsDb'
import { useClientType } from '@renderer/composables/useClientType'
import { useCommandsDb } from '@renderer/composables/useCommandsDb'
import { useControls } from '@renderer/composables/useControls'
import { useShareSettings } from '@renderer/composables/useShareSettings'
import { useStopEverything } from '@renderer/composables/useStopEverything'
import { useTheme } from '@renderer/composables/useTheme'
import { useViewerGroups } from '@renderer/composables/useViewerGroups'
import { serverWsUrl } from '@renderer/composables/useWebsocketSettings'
import type { ServerStatus } from '@renderer/lib/tauri-bridge'
import { api } from '@renderer/lib/tauri-bridge'
import { checkClient } from '@renderer/utils/checkClient'
import { AUTHENTICATED_MAX_PER_WINDOW, isRateLimited } from '@renderer/utils/rateLimit'
import type { ClientListEntry, ControlTypes, ViewerGroupsMessage } from '@toolkitosc/shared-ui'
import {
  INTIFACE_PATTERN_OFF_ID,
  isOptionDisabled,
  limitRange,
  PROTOCOL_VERSION,
  resolveControlForViewer,
  sanitizeCommand,
  useIntifaceControl,
  useIntifacePatternControl,
  useOpenShockControl,
  VIEWER_LIVE_ID,
  VIEWER_PATTERN_ID
} from '@toolkitosc/shared-ui'
import type { WebSocketStatus } from '@vueuse/core'
import type { ComputedRef } from 'vue'
import { watch } from 'vue'
import { computed, ref } from 'vue'

export type Client = ClientListEntry

export const clients = ref<Client[]>([])

// `clients` has one entry per connection, so the same person on several browsers/tabs shows up
// more than once - this counts people instead, keyed the same way the Viewers page groups them.
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
  const { controls, visibleControls, setControlLastUser, getControl, isGroupHidden, handleCommand } =
    useControls(() => sendControls())


  const { token, user, setToken } = useAuth()
  const { avatarDetails } = useAvatarDetails()

  const addressList = computed<Set<string>>(() => {
    const addresses = new Set<string>()

    // visibleControls, not controls: a hidden group's addresses must never be forwarded to
    // clients either, or its live OSC state would leak to the share page even though the group
    // itself doesn't.
    visibleControls.value?.forEach((group) => {
      group.controls.forEach((control) => {
        if (control.type === 'boolean') {
          addresses.add(control.inputAddress)
        } else if (control.type === 'boolean-group') {
          control.inputs.forEach((input) => {
            addresses.add(input.inputAddress)
          })
        } else if (control.type === 'boolean-enum') {
          control.inputs.forEach((input) => {
            input.inputAddress.true.forEach((e) => addresses.add(e))
            input.inputAddress.false.forEach((e) => addresses.add(e))
          })
        } else if (
          control.type === 'enum' ||
          control.type === 'slider' ||
          control.type === 'step-enum'
        ) {
          addresses.add(control.inputAddress)
        }
      })
    })

    return addresses
  })

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
  const { isBanned } = useBannedClientsDb()
  const { add: addCommandToDb, update: updateCommandInDb } = useCommandsDb()
  const { lastUpdate } = useOpenShockControl()
  const { lastUpdate: intifaceLastUpdate, controlValue: intifaceValues } = useIntifaceControl()
  const { lastUpdate: intifacePatternLastUpdate, controlValue: intifacePatternValues } = useIntifacePatternControl()
  const { clientType } = useClientType()
  const { shareSettings, showViewers } = useShareSettings()
  const { announceAction } = useChatbox()
  const { paused } = useStopEverything()
  const { viewerGroups, activeGroupIds } = useViewerGroups()
  const { selectedPrimary, selectedSecondary } = useTheme()

  const status = computed<WebSocketStatus>(() =>
    serverStatus.value === 'open' ? 'OPEN' : serverStatus.value === 'connecting' ? 'CONNECTING' : 'CLOSED'
  )
  // From StatusBar's mount to its unmount. Rust retries a dropped link by itself, signs in with the
  // token and pings. A new server URL or token connects again from here.
  let active = false

  const connect = async (): Promise<void> => {
    await listening
    if (active) await api.serverConnect(serverWsUrl.value, token.value, PROTOCOL_VERSION)
  }

  const open = (): void => {
    active = true
    void connect()
  }

  const close = (): void => {
    active = false
    void api.serverDisconnect()
  }

  watch([serverWsUrl, token], () => void connect())

  onServerDebug = (text) => logMessage('⇧', JSON.parse(text))

  onServerText = (text) => {
    const data = JSON.parse(text)

    logMessage('⇩', data)

    if (data.type === 'auth-success') {
      serverProtocolVersion.value = data.message.serverVersion
      serverSupportedControlTypes.value = data.message.supportedControlTypes
      protocolMismatchReason.value = null
      useAiAccess().applyUpdate(data.message.aiAccess)
      sendControls()
      sendTheme()
      sendMessage('share-settings', shareSettings.value)
      sendMessage('pause-state', { paused: paused.value })
      sendMessage('viewer-groups', viewerGroupsMessage.value)
    } else if (data.type === 'auth-error') {
      // Rust stops retrying with this token. Clearing it connects again, signed out.
      console.error('Authentication failed:', data.message)
      setToken(undefined)
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
    } else if (data.type === 'command' && paused.value) {
      // Stop everything: viewer commands are dropped, not logged, until the host resumes. The
      // relay drops them too, and the share page shows a paused card.
    } else if (data.type === 'command') {
      const client = clientsMap.value[data.from]

      if (client && data.message.groupId && data.message.controlId) {
        // Controls are avatar-specific, so a control may legitimately fail to resolve here -
        // e.g. the host hasn't loaded that avatar's controls yet, or the client is a step behind
        // after an avatar switch. Still log the attempt (using the info the message itself
        // carries) so it shows up in the client's history; only skip execution, which does need
        // a resolved control to know the OSC address/type. A control that *does* resolve but is
        // explicitly locked - or marked unavailable, e.g. an OpenShock control while no valid
        // API key is configured (see useControls.ts) - is still ignored entirely, same as before.
        // Same for a control whose group has been hidden client-side: `sendControls` already
        // keeps the server/viewers from ever learning it exists, but a viewer that cached an
        // older `controls-update` (or a hand-crafted command) could still send one, so this is
        // enforced host-side too rather than trusted to never arrive.
        // sanitizeCommand also applies the active profile's limits: a blocked option or a value
        // of the wrong type is dropped the same way, and slider/toy values are clamped to the
        // allowed range. The relay already did this, but the host has the final say.
        // The viewer's groups can give them more than the profile gives everyone.
        const viewer = describeViewer(client)
        const control = getControl(data.message.groupId, data.message.controlId)
        const command = control ? sanitizeCommand(resolveControlForViewer(control, activeGroupIds(viewer.key)), data.message) : undefined
        const { ip } = viewer
        // A viewer's own pattern, or the live value while they record it (see viewerPatterns on
        // the control). The host can keep it to viewers signed in with Discord.
        const viewerPattern = command?.type === 'intiface-pattern' && (command.value === VIEWER_PATTERN_ID || command.value === VIEWER_LIVE_ID)
        const viewerPatternBlocked =
          viewerPattern && control?.type === 'intiface-pattern' && !!control.viewerPatterns?.signedInOnly && !client.user?.discord?.id

        if (command !== null && !viewerPatternBlocked && !isGroupHidden(data.message.groupId)) {
          const banned = isBanned(ip, viewer.discordId)
          const clientValid = checkClient(clientType.value, client)

          if (banned) {
            sendMessage('client-banned', {
              peerId: client.peerId,
              scope: banned.scope,
              reason: banned.reason
            })
          } else if (!clientValid) {
            sendMessage('client-invalid', {
              peerId: client.peerId,
              reason: 'Client failed validation',
              type: clientType.value
            })
          } else if (
            isRateLimited(
              client.peerId,
              client.user?.discord?.id ? AUTHENTICATED_MAX_PER_WINDOW : undefined
            )
          ) {
            // Deliberately no added delay/throttle before this check - a dragged slider's own
            // displayed position depends on this exact round trip (see ControlSlider.vue's
            // `get()`), so anything that buffers a command before executing it makes dragging
            // feel laggy for whoever's holding it. The slider's outgoing rate is instead kept
            // low at the true source (see that component's own debounce), not sampled down
            // again here.
            sendMessage('rate-limited', {
              peerId: client.peerId,
              reason: 'You are sending commands too quickly - slow down.'
            })
          } else if (command?.type === 'intiface-pattern' && command.value === VIEWER_LIVE_ID) {
            // Live values while recording only move the toy: nothing to log, announce or show.
            if (control) handleCommand(control, command)
          } else {
            if (control) {
              setControlLastUser(data.message.groupId, data.message.controlId, {
                avatar: viewer.avatar,
                displayName: viewer.displayName,
                discordId: viewer.discordId ?? undefined,
                ip
              })
            }

            // For an open-shock-shocker command, the intensity/duration/shockers actually applied
            // are randomized inside handleCommand and weren't known yet when this message arrived
            // - run it first and log its result instead of the (always absent) message value.
            // A viewer's own pattern is logged with its samples, and its row gets the time it stopped.
            const samples = command?.type === 'intiface-pattern' && command.value === VIEWER_PATTERN_ID ? command.samples : undefined
            // Filled in once the row is written, which is after the pattern starts.
            const logged: { id?: Promise<number | undefined> } = {}
            const onPatternEnd = (): void => {
              const endedAt = Date.now()

              void logged.id?.then((id) => id !== undefined && updateCommandInDb(id, { endedAt }))
            }
            const openShockResult =
              control && command
                ? handleCommand(control, command, { viewerName: showViewers.value ? viewer.displayName : undefined, onPatternEnd: samples ? onPatternEnd : undefined })
                : undefined
            const sent = command ?? data.message
            const accepted = {
              groupId: data.message.groupId,
              controlId: data.message.controlId,
              controlName: control?.name || data.message.controlName || data.message.controlId,
              type: data.message.type,
              value: openShockResult ?? ('value' in sent ? sent.value : undefined)
            }

            logged.id = addCommandToDb({ ...accepted, samples, discordId: viewer.discordId ?? undefined, peerId: client.peerId, ip })
            // Shown to every viewer as the share page's last action (see the server's host.ts).
            sendMessage('command-accepted', { ...accepted, peerId: client.peerId })
            // And to the instance, in VRChat's chatbox, if the host turned it on. Names follow
            // the share page's own setting.
            announceAction({
              ...accepted,
              viewerKey: viewer.key,
              viewerName: showViewers.value ? viewer.displayName : null,
              control
            })
            // Automations can start from it. A shock that didn't go out (cooldown) doesn't count.
            if (control && (control.type !== 'open-shock-shocker' || openShockResult)) {
              void api.automationEvent({ type: 'viewer-command', controlId: control.id, controlType: control.type })
            }
          }
        }
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

  // Rust sends the current value of each of these addresses after the controls, then every change.
  const sendControls = (): void => {
    sendMessage('controls-update', { avatarId: avatarDetails.value?.id ?? null, groups: visibleControls.value })
    void api.serverSetForwardedAddresses(Array.from(addressList.value))
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

  watch([selectedPrimary, selectedSecondary], () => sendTheme())

  watch(shareSettings, (settings) => {
    if (status.value === 'OPEN') sendMessage('share-settings', settings)
  })

  watch(paused, (value) => {
    if (status.value === 'OPEN') sendMessage('pause-state', { paused: value })
  })

  // Who on the share page is in which viewer group, so the server sends each of them their own
  // copy of the controls. Changes when someone joins or leaves, is added to or removed from a
  // group, a temporary membership ends, or a group is renamed or recolored.
  const viewerGroupsMessage = computed<ViewerGroupsMessage>(() => {
    const members: Record<string, string[]> = {}

    clients.value.forEach((client) => {
      const ids = activeGroupIds(describeViewer(client).key)

      if (ids.length) members[client.peerId] = ids
    })

    return { groups: viewerGroups.value, members }
  })

  watch(viewerGroupsMessage, (message, previous) => {
    if (status.value === 'OPEN' && JSON.stringify(message) !== JSON.stringify(previous)) sendMessage('viewer-groups', message)
  })

  // When a profile turns on, or its limits change while it's on, bring running toys within the new
  // limits: a toy above its new max drops to it, and a pattern that's now blocked stops. Only ever
  // lowers, so it can't start anything; slider and enum avatar state is left alone. Uses the most
  // a viewer group gets, so a friend's toy isn't cut to what everyone else may set.
  watch(
    () =>
      controls.value
        .flatMap((group) => group.controls)
        .map((control) => [control, resolveControlForViewer(control, Object.keys(control.groupRules ?? {})).limits] as const),
    (entries, previous) => {
      const before = new Map(previous?.map(([control, limits]) => [control.id, JSON.stringify(limits)]))

      entries.forEach(([control, limits]) => {
        if (!limits || before.get(control.id) === JSON.stringify(limits)) return

        if (control.type === 'intiface-toy') {
          const { max } = limitRange(limits)

          if ((intifaceValues.value.get(control.id) ?? 0) > max) handleCommand(control, { type: 'intiface-toy', value: max })
        } else if (control.type === 'intiface-pattern') {
          const current = intifacePatternValues.value.get(control.id)

          // A viewer's own pattern was clamped to the old limits when it started, so it stops too.
          if (current && current !== INTIFACE_PATTERN_OFF_ID && (current === VIEWER_PATTERN_ID || isOptionDisabled(limits, current))) {
            handleCommand(control, { type: 'intiface-pattern', value: INTIFACE_PATTERN_OFF_ID })
          }
        }
      })
    }
  )

  return { clients, status, sendMessage, open, close }
}
