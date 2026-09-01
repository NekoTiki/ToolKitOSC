import type { Client } from '@renderer/db/clients.db'
import { ref } from 'vue'

export type LogClient = Omit<Client, 'createdAt' | 'uniqueKey'>

const open = ref(false)
const client = ref<LogClient>()

export function useClientLogsModal(): {
  open: typeof open
  client: typeof client
  openModal: (target: LogClient) => void
} {
  const openModal = (target: LogClient): void => {
    client.value = target
    open.value = true
  }

  return { open, client, openModal }
}
