import { useAuth } from '@renderer/composables/useAuth'
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
import { getStableIp } from '@renderer/utils/stableIp'
import { useOpenShockControl } from '@vrc-osc-toolkit/shared-ui'
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
const clientsMap = computed<Record<string, Client>>(() => {
  const map: Record<string, Client> = {}

  clients.value.forEach((client) => (map[client.peerId] = client))

  return map
})

export const getGuestAvatar = (peerId: string, username: string = 'Guest'): string =>
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
  const { controls, setControlLastUser, getControl, handleCommand } = useControls(() =>
    sendControls()
  )

  watch(
    controls.value,
    () => {
      console.log('Controls changed')
    },
    { deep: true }
  )

  const { increaseIpCount, increaseDiscordIdCount } = useCommandAnalytics()
  const { token, user, setToken } = useAuth()

  const addressList = computed<Set<string>>(() => {
    const addresses = new Set<string>()

    controls.value?.forEach((group) => {
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

  const { args } = useOscMessages((msg) => {
    if (!addressList.value.has(msg.address)) return

    sendMessage('args-update', msg)
  })

  const { onlineClients, add: addClient } = useClientsDb()
  const { isBanned } = useBannedClientsDb()
  const { add: addCommandToDb } = useCommandsDb()
  const { lastUpdate } = useOpenShockControl()
  const { clientType } = useClientType()
  const { selectedPrimary, selectedSecondary } = useTheme()

  const { status, data, send, open, close } = useWebSocket(() => `${serverWsUrl.value}/host`, {
    heartbeat: {
      message: 'ping',
      interval: 5000,
      pongTimeout: 5000
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
        sendControls()
        sendTheme()
      } else if (data.type === 'auth-error') {
        console.error('Authentication failed:', data.message)
        setToken(undefined)
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
              getGuestAvatar(client.peerId, client.user?.userDefinedDisplayName || 'Guest'),
            discordId: client.user?.discord?.id || null,
            displayName:
              client.user?.discord?.name ||
              client.user?.username ||
              client.user?.userDefinedDisplayName ||
              'Guest',
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
          // explicitly locked is still ignored entirely, same as before.
          const control = getControl(data.message.groupId, data.message.controlId)
          const ip = getStableIp(client.ip)

          if (!control?.locked) {
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
            } else {
              increaseIpCount(ip)
              if (client.user?.discord?.id) increaseDiscordIdCount(client.user.discord?.id)

              if (control) {
                setControlLastUser(data.message.groupId, data.message.controlId, {
                  avatar:
                    client.user?.discord?.avatar ||
                    getGuestAvatar(client.peerId, client.user?.userDefinedDisplayName || 'Guest'),
                  displayName:
                    client.user?.discord?.name ||
                    client.user?.username ||
                    client.user?.userDefinedDisplayName ||
                    'Guest',
                  discordId: client.user?.discord?.id,
                  ip
                })
              }

              addCommandToDb({
                groupId: data.message.groupId,
                controlId: data.message.controlId,
                controlName: control?.name || data.message.controlName || data.message.controlId,
                type: data.message.type,
                value: 'value' in data.message ? data.message.value : undefined,
                discordId: client.user?.discord?.id,
                peerId: client.peerId,
                ip
              })

              if (control) handleCommand(control, data.message)
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
              getGuestAvatar(client.peerId, client.user?.userDefinedDisplayName || 'Guest'),
            discordId: client.user?.discord?.id || null,
            displayName:
              client.user?.discord?.name ||
              client.user?.username ||
              client.user?.userDefinedDisplayName ||
              'Guest',
            ip
          })
        })
      }
    }
  })

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

  const initMessages = (): void => {
    sendMessage('auth-token', { token: token.value })
  }

  const sendControls = (): void => {
    sendMessage('controls-update', controls.value)
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

  watch([selectedPrimary, selectedSecondary], () => sendTheme())

  return { clients, status, data, sendMessage, send, open, close }
}
