import ClientLogsModal from '@renderer/components/ClientLogsModal.vue'
import type { Client } from '@renderer/db/clients.db'

export type LogClient = Omit<Client, 'createdAt' | 'uniqueKey'>

const overlay = useOverlay()
const modal = overlay.create(ClientLogsModal)

export function useClientLogsModal(): { openModal: (target: LogClient) => void } {
  return { openModal: (target) => modal.open({ client: target }) }
}
