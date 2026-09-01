import type { ClientType } from '@renderer/composables/useClientType'
import type { Client } from '@renderer/composables/useWebsocketHost'

export const checkClient = (type: ClientType, client: Client): boolean => {
  if (type === 'discord') return !!client.user?.discord?.id

  if (type === 'username') {
    return !!client.user?.userDefinedDisplayName || !!client.user?.discord?.id
  }

  return true
}
