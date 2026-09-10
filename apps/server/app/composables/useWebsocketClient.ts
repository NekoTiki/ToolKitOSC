import type { ControlGroup, ServerToViewerMessage } from '@vrc-osc-toolkit/shared-ui'

import { useClientTheme } from '~/composables/useClientTheme'
import { useIntifaceControl } from '~/composables/useIntifaceControl'
import { useIntifacePatternControl } from '~/composables/useIntifacePatternControl'
import { useOpenShockControl } from '~/composables/useOpenShockControl'

export function useWebsocketClient(roomId: string) {
  const sendMessage = (type: string, message: unknown) => {
    console.log(
      `%c[⇧ WS]%c[${roomId}]%c[${type}] %c${JSON.stringify(message)}`,
      'color: #42b983; font-weight: bold;',
      'color: #ba43ba; font-weight: bold;',
      'color: #437cba; font-weight: bold;',
      'color: #ffffff;'
    )

    send(JSON.stringify({ type, message }))
  }

  const { args } = useOscMessages()
  const { setValue } = useOpenShockControl()
  const { setValue: setIntifaceValue } = useIntifaceControl()
  const { setValue: setIntifacePatternValue } = useIntifacePatternControl()
  const { setTheme } = useClientTheme()
  const controlGroups = ref<ControlGroup[]>([])
  const hostStatus = ref<'online' | 'offline'>('offline')
  const authRequired = ref<'username' | 'discord' | null>(null)
  const banned = ref<{ scope: 'ip' | 'discord'; reason?: string } | null>(null)

  const { status, data, send, open, close } = useWebSocket(`/ws/${roomId}`, {
    heartbeat: {
      message: 'ping',
      interval: 5000,
      pongTimeout: 5000
    },
    immediate: false,
    onMessage: (ws, ev) => {
      if (ev.data === 'pong') return

      const data = JSON.parse(ev.data) as ServerToViewerMessage

      console.log(
        `%c[⇩ WS]%c[${roomId}]%c[${data.type}] %c${JSON.stringify(data.message)}`,
        'color: #42b983; font-weight: bold;',
        'color: #ba43ba; font-weight: bold;',
        'color: #437cba; font-weight: bold;',
        'color: #ffffff;'
      )

      if (data.type === 'controls-update') {
        controlGroups.value = data.message
      } else if (data.type === 'args-initial') {
        args.value = data.message
      } else if (data.type === 'args-update') {
        if (args.value) {
          args.value[data.message.address] = data.message.args
        }
      } else if (data.type === 'host-status') {
        hostStatus.value = data.message
      } else if (data.type === 'open-shock-value-update') {
        setValue(data.message.controlId, data.message.value)
      } else if (data.type === 'intiface-value-update') {
        setIntifaceValue(data.message.controlId, data.message.value)
      } else if (data.type === 'intiface-pattern-value-update') {
        setIntifacePatternValue(data.message.controlId, data.message.value)
      } else if (data.type === 'client-invalid') {
        // The wire type is the full ClientType union ('everyone' | 'username' | 'discord'), but
        // checkClient() (see apps/client/src/utils/checkClient.ts) never fails validation when
        // the host's policy is 'everyone', so client-invalid.type in practice is always
        // 'username' | 'discord' — the only two AuthModal actually renders a flow for.
        authRequired.value = data.message.type as 'username' | 'discord'
      } else if (data.type === 'client-banned') {
        banned.value = { scope: data.message.scope, reason: data.message.reason }
      } else if (data.type === 'theme-update') {
        if (data.message.primary) setTheme('primary', data.message.primary)
        if (data.message.secondary) setTheme('secondary', data.message.secondary)
      }
    }
  })

  return {
    controlGroups,
    authRequired,
    banned,
    status,
    hostStatus,
    data,
    send,
    sendMessage,
    open,
    close
  }
}
