import { useAiAccess } from '@renderer/composables/useAiAccess'
import { useAiCredits } from '@renderer/composables/useAiCredits'
import { useAiGenerationStatus } from '@renderer/composables/useAiGenerationStatus'
import { useAuth } from '@renderer/composables/useAuth'
import { useAvatarDetails } from '@renderer/composables/useAvatarDetails'
import { useBannedClientsDb } from '@renderer/composables/useBannedClientsDb'
import { formatUniqueKey, useClientsDb } from '@renderer/composables/useClientsDb'
import { useClientType } from '@renderer/composables/useClientType'
import { useCommandAnalytics } from '@renderer/composables/useCommandAnalytics'
import { useCommandsDb } from '@renderer/composables/useCommandsDb'
import { useControls } from '@renderer/composables/useControls'
import { useOscMessages } from '@renderer/composables/useOscMessages'
import { useTheme } from '@renderer/composables/useTheme'
import { serverWsUrl } from '@renderer/composables/useWebsocketSettings'
import { checkClient } from '@renderer/utils/checkClient'
import { AUTHENTICATED_MAX_PER_WINDOW, isRateLimited } from '@renderer/utils/rateLimit'
import { getStableIp } from '@renderer/utils/stableIp'
import type { ControlTypes } from '@toolkitosc/shared-ui'
import {
  guestName,
  PROTOCOL_VERSION,
  useIntifaceControl,
  useIntifacePatternControl,
  useOpenShockControl
} from '@toolkitosc/shared-ui'
import type { WebSocketStatus } from '@vueuse/core'
import { useWebSocket } from '@vueuse/core'
import type { Ref, ShallowRef } from 'vue'
import { watch } from 'vue'
import { computed, onMounted, ref } from 'vue'

export type Client = {
  user: {
    discord?: {
      id: string
      avatar: string
      name: string
    }
    userDefinedDisplayName?: string
    username?: string
    email?: string
  } | null
  peerId: string
  ip: string
}

export const clients = ref<Client[]>([])

// Mirrors useWebSocket's own `status` at module scope, same reasoning as `clients` above: the
// actual socket is only ever opened once (by StatusBar.vue, the only place useWebsocketHost() may be
// called - every call opens its own socket, and two host sockets for one account keep kicking each
// other off the server), but other places - e.g. Settings → Account's server status line - need to read the current
// connection state without opening a second connection of their own.
export const hostStatus = ref<WebSocketStatus>('CLOSED')
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

export const getGuestAvatar = (peerId: string, username: string = guestName(peerId)): string =>
  `https://ui-avatars.com/api/?name=${username}&background=random&size=256&${peerId}`

export function useWebsocketHost(): {
  clients: typeof clients
  status: ShallowRef<WebSocketStatus>
  data: Ref<unknown, unknown>
  sendMessage: (type: string, message: unknown) => void
  send: (data: string | ArrayBuffer | Blob, useBuffer?: boolean) => boolean
  open: () => void
  close: WebSocket['close']
} {
  const { visibleControls, setControlLastUser, getControl, isGroupHidden, handleCommand } =
    useControls(() => sendControls())


  const { increaseIpCount, increaseDiscordIdCount } = useCommandAnalytics()
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

  const { args } = useOscMessages((msg) => {
    if (!addressList.value.has(msg.address)) return

    sendMessage('args-update', msg)
  })

  const { onlineClients, add: addClient } = useClientsDb()
  const { isBanned } = useBannedClientsDb()
  const { add: addCommandToDb } = useCommandsDb()
  const { lastUpdate } = useOpenShockControl()
  const { lastUpdate: intifaceLastUpdate } = useIntifaceControl()
  const { lastUpdate: intifacePatternLastUpdate } = useIntifacePatternControl()
  const { clientType } = useClientType()
  const { selectedPrimary, selectedSecondary } = useTheme()

  const { status, data, send, open, close } = useWebSocket(() => `${serverWsUrl.value}/host`, {
    heartbeat: {
      message: 'ping',
      interval: 5000,
      pongTimeout: 5000
    },
    // vueuse's useWebSocket does NOT retry on its own unless this is set - without it, a dropped
    // connection (server restart, network blip) just sits disconnected forever until the app is
    // restarted. Unlimited retries: this is the host's only link to the relay server, so there's
    // no point giving up.
    autoReconnect: {
      retries: -1,
      delay: 3000
    },
    immediate: false,
    onConnected: () => initMessages(),
    onMessage: (_ws, ev) => {
      if (ev.data === 'pong') return

      const data = JSON.parse(ev.data)

      console.log(
        `%c[⇩ WS]%c[${user.value?.discord.id}]%c[${data.type}] %c${JSON.stringify(data.message)}`,
        'color: #42b983; font-weight: bold;',
        'color: #ba43ba; font-weight: bold;',
        'color: #437cba; font-weight: bold;',
        'color: #ffffff;'
      )

      if (data.type === 'auth-success') {
        serverProtocolVersion.value = data.message.serverVersion
        serverSupportedControlTypes.value = data.message.supportedControlTypes
        protocolMismatchReason.value = null
        useAiAccess().applyUpdate(data.message.aiAccess)
        sendControls()
        sendTheme()
      } else if (data.type === 'auth-error') {
        console.error('Authentication failed:', data.message)
        setToken(undefined)
        close()
      } else if (data.type === 'protocol-mismatch') {
        console.error('Protocol mismatch:', data.message.reason)
        serverProtocolVersion.value = data.message.serverVersion
        protocolMismatchReason.value = data.message.reason
        useToast().add({
          title: 'App update required',
          description: data.message.reason,
          color: 'error'
        })
        // Not an auth problem - don't clear the token. `close()` marks the connection as
        // explicitly closed, which stops vueuse's autoReconnect from retrying forever against a
        // server it can never satisfy until the app is updated (mirrors auth-error above).
        close()
      } else if (data.type === 'update-username') {
        const client = clientsMap.value[data.from]

        if (client) {
          const ip = getStableIp(client.ip)

          if (!client.user) {
            client.user = {
              userDefinedDisplayName: data.message.displayName
            }
          } else client.user.userDefinedDisplayName = data.message.displayName

          addClient({
            avatar:
              client.user?.discord?.avatar ||
              getGuestAvatar(client.peerId, client.user?.userDefinedDisplayName || guestName(client.peerId)),
            discordId: client.user?.discord?.id || null,
            displayName:
              client.user?.discord?.name ||
              client.user?.username ||
              client.user?.userDefinedDisplayName ||
              guestName(client.peerId),
            ip
          })
        }
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
          const control = getControl(data.message.groupId, data.message.controlId)
          const ip = getStableIp(client.ip)

          if (!control?.locked && !control?.unavailable && !isGroupHidden(data.message.groupId)) {
            const banned = isBanned(ip, client.user?.discord?.id)
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
            } else {
              increaseIpCount(ip)
              if (client.user?.discord?.id) increaseDiscordIdCount(client.user.discord?.id)

              if (control) {
                setControlLastUser(data.message.groupId, data.message.controlId, {
                  avatar:
                    client.user?.discord?.avatar ||
                    getGuestAvatar(client.peerId, client.user?.userDefinedDisplayName || guestName(client.peerId)),
                  displayName:
                    client.user?.discord?.name ||
                    client.user?.username ||
                    client.user?.userDefinedDisplayName ||
                    guestName(client.peerId),
                  discordId: client.user?.discord?.id,
                  ip
                })
              }

              // For an open-shock-shocker command, the intensity/duration/shockers actually applied
              // are randomized inside handleCommand and weren't known yet when this message arrived
              // - run it first and log its result instead of the (always absent) message value.
              const openShockResult = control ? handleCommand(control, data.message) : undefined

              addCommandToDb({
                groupId: data.message.groupId,
                controlId: data.message.controlId,
                controlName: control?.name || data.message.controlName || data.message.controlId,
                type: data.message.type,
                value:
                  openShockResult ?? ('value' in data.message ? data.message.value : undefined),
                discordId: client.user?.discord?.id,
                peerId: client.peerId,
                ip
              })
            }
          }
        }
      } else if (data.type === 'client-list') {
        clients.value = data.message

        onlineClients.value = data.message.map((client: Client) => {
          const ip = getStableIp(client.ip)

          return formatUniqueKey(ip, client.user?.discord?.id || null)
        })

        data.message.forEach((client: Client) => {
          const ip = getStableIp(client.ip)

          addClient({
            avatar:
              client.user?.discord?.avatar ||
              getGuestAvatar(client.peerId, client.user?.userDefinedDisplayName || guestName(client.peerId)),
            discordId: client.user?.discord?.id || null,
            displayName:
              client.user?.discord?.name ||
              client.user?.username ||
              client.user?.userDefinedDisplayName ||
              guestName(client.peerId),
            ip
          })
        })
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
  })

  watch(
    status,
    (value) => {
      hostStatus.value = value
      // Nobody can reach your controls while this socket is down, so nobody is online - this also
      // stamps everyone's "last seen" (see useClientsDb.ts). The server resends the list on
      // reconnect.
      if (value !== 'OPEN') onlineClients.value = []
    },
    { immediate: true }
  )

  onMounted(() => open())

  const sendMessage = (type: string, message: unknown): void => {
    console.log(
      `%c[⇧ WS]%c[${user.value?.discord.id}]%c[${type}] %c${JSON.stringify(message)}`,
      'color: #42b983; font-weight: bold;',
      'color: #ba43ba; font-weight: bold;',
      'color: #437cba; font-weight: bold;',
      'color: #ffffff;'
    )

    send(JSON.stringify({ type, message }))
  }

  // Signed out (or the token not loaded yet): nothing to authenticate with - the watch below sends
  // it once a token arrives on an open socket.
  const initMessages = (): void => {
    if (!token.value) return

    sendMessage('auth-token', { token: token.value, protocolVersion: PROTOCOL_VERSION })
  }

  watch(token, (value) => {
    if (value && status.value === 'OPEN') initMessages()
  })

  const sendControls = (): void => {
    sendMessage('controls-update', { avatarId: avatarDetails.value?.id ?? null, groups: visibleControls.value })
    Array.from(addressList.value).forEach((address) => {
      const argList = args.value?.[address]

      if (argList) sendMessage('args-update', { address, args: argList })
    })
  }

  const sendTheme = (): void => {
    sendMessage('theme-update', {
      primary: selectedPrimary.value,
      secondary: selectedSecondary.value
    })
  }

  watch(
    () => lastUpdate.value,
    (newUpdate) => {
      if (!newUpdate) return

      sendMessage('open-shock-value-update', newUpdate)
    }
  )

  watch(
    () => intifaceLastUpdate.value,
    (newUpdate) => {
      if (!newUpdate) return

      sendMessage('intiface-value-update', newUpdate)
    }
  )

  watch(
    () => intifacePatternLastUpdate.value,
    (newUpdate) => {
      if (!newUpdate) return

      sendMessage('intiface-pattern-value-update', newUpdate)
    }
  )

  watch([selectedPrimary, selectedSecondary], () => sendTheme())

  return { clients, status, data, sendMessage, send, open, close }
}
