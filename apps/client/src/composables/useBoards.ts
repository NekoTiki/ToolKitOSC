import type {
  Board,
  BoardPairingEvent,
  BoardsSnapshot,
  FoundBoard
} from '@renderer/lib/tauri-bridge'
import { api } from '@renderer/lib/tauri-bridge'
import type { ComputedRef, Ref } from 'vue'
import { computed, ref } from 'vue'

// ESP32 boards. The backend owns all of it (discovery, pairing, the link to each board) and sends
// the whole list on every change, so this only mirrors it.
const snapshot = ref<BoardsSnapshot>({ boards: [], found: [] })
// The pairing in progress, or how the last one ended. null once dismissed.
const pairing = ref<BoardPairingEvent | null>(null)
// When the board stops waiting for its BOOT press, for the countdown.
const pairingDeadline = ref<number | null>(null)

// Registered at module scope, like useAvatarDetails.ts, so no event is missed while a component
// is still mounting.
api.onBoardsChanged((value) => (snapshot.value = value))
api.onBoardPairing((event) => {
  pairing.value = event
  pairingDeadline.value = event.state === 'waiting' ? Date.now() + event.timeoutMs : null
})
void api.boardsList().then((value) => (snapshot.value = value))

export function useBoards(): {
  boards: ComputedRef<Board[]>
  // Boards on the network that aren't added yet.
  found: ComputedRef<FoundBoard[]>
  pairing: Ref<BoardPairingEvent | null>
  pairingDeadline: Ref<number | null>
  pairingActive: ComputedRef<boolean>
  add: (id: string) => Promise<void>
  addByAddress: (address: string) => Promise<void>
  cancelPairing: () => void
  dismissPairing: () => void
  remove: (id: string) => Promise<void>
} {
  const boards = computed(() => snapshot.value.boards)
  const found = computed(() => snapshot.value.found.filter((board) => !board.added))
  const pairingActive = computed(
    () => pairing.value?.state === 'connecting' || pairing.value?.state === 'waiting'
  )

  // The outcome (paired, failed, cancelled) arrives as a pairing event too, which is what the UI
  // shows, so the rejection itself is only swallowed here.
  const add = (id: string): Promise<void> =>
    api.boardAdd(id).then(
      () => undefined,
      () => undefined
    )

  const addByAddress = (address: string): Promise<void> =>
    api.boardAddByAddress(address).then(
      () => undefined,
      () => undefined
    )

  const cancelPairing = (): void => {
    void api.boardPairCancel()
  }

  const dismissPairing = (): void => {
    pairing.value = null
  }

  const remove = (id: string): Promise<void> => api.boardRemove(id)

  return {
    boards,
    found,
    pairing,
    pairingDeadline,
    pairingActive,
    add,
    addByAddress,
    cancelPairing,
    dismissPairing,
    remove
  }
}
