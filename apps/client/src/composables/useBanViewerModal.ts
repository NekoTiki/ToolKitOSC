import BanViewerModal from '@renderer/components/BanViewerModal.vue'
import type { BanScope, Client } from '@renderer/db/clients.db'

// Created lazily from the first useBanViewerModal() call - useOverlay() injects, so it has to run
// inside a component's setup(), not at module import.
let modal: ReturnType<ReturnType<typeof useOverlay>['create']> | undefined

// Resolves with the chosen scope and reason, or false if cancelled.
export function useBanViewerModal(): {
  openModal: (client: Pick<Client, 'displayName' | 'ip' | 'discordId'>) => Promise<{ scope: BanScope; reason?: string } | false>
} {
  modal ??= useOverlay().create(BanViewerModal)

  return { openModal: (client) => modal!.open({ client }).result as Promise<{ scope: BanScope; reason?: string } | false> }
}
