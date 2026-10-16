// ESP32 boards - src-tauri/src/boards/.
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'

// ESP32 boards - mirrors src-tauri/src/boards/mod.rs. 'needs-pairing': the board refused its token
// (reset, or paired to another PC since). 'update-firmware': it speaks another protocol version.
export type BoardStatus = 'connecting' | 'online' | 'offline' | 'needs-pairing' | 'update-firmware'

export interface BoardOutputPin {
  pin: number
  pwm: boolean
  // Silkscreen name, when the board's profile knows it.
  label?: string
}

// One output set up in the app ("Fan on GPIO 4"). The label stays in the app.
export interface BoardOutputConfig {
  pin: number
  label: string
  // For relay modules that switch on when the pin is LOW.
  activeLow: boolean
  maxOnMs: number
}

// A board refused something (see the firmware's PROTOCOL.md for the codes).
export interface BoardError {
  id: string
  code: string
  pin: number | null
}

export interface Board {
  id: string
  name: string
  host: string
  port: number
  fw: string | null
  chip: string | null
  board: string | null
  // Pins the board offers as outputs.
  outputs: BoardOutputPin[]
  // The outputs set up in the app, null until first saved.
  config: BoardOutputConfig[] | null
  status: BoardStatus
  // Unix ms, while offline.
  offlineSince: number | null
}

export interface FoundBoard {
  id: string
  name: string
  host: string
  port: number
  fw: string | null
  proto: number | null
  // Some app holds a token for it.
  paired: boolean
  // Already in this app's list.
  added: boolean
}

export interface BoardsSnapshot {
  boards: Board[]
  found: FoundBoard[]
}

export type BoardPairingEvent =
  | { state: 'connecting'; name: string }
  | { state: 'waiting'; name: string; timeoutMs: number }
  | { state: 'paired'; id: string; name: string }
  | { state: 'failed'; reason: string }
  | { state: 'cancelled' }

export const boardsApi = {
  boardsList: (): Promise<BoardsSnapshot> => invoke('boards_list'),
  // Both resolve once someone pressed the board's BOOT button, and reject with a message to show.
  boardAdd: (id: string): Promise<Board> => invoke('board_add', { id }),
  boardAddByAddress: (address: string): Promise<Board> =>
    invoke('board_add_by_address', { address }),
  boardPairCancel: (): Promise<void> => invoke('board_pair_cancel'),
  boardRemove: (id: string): Promise<void> => invoke('board_remove', { id }),
  boardSaveOutputs: (id: string, outputs: BoardOutputConfig[]): Promise<void> =>
    invoke('board_save_outputs', { id, outputs }),
  boardTestOutput: (id: string, pin: number): Promise<void> =>
    invoke('board_test_output', { id, pin }),
  boardsStopAll: (): Promise<void> => invoke('boards_stop_all'),
  onBoardError: (callback: (error: BoardError) => void): void => {
    void listen<BoardError>('board-error', (event) => callback(event.payload))
  },
  onBoardsChanged: (callback: (snapshot: BoardsSnapshot) => void): void => {
    void listen<BoardsSnapshot>('boards-changed', (event) => callback(event.payload))
  },
  onBoardPairing: (callback: (event: BoardPairingEvent) => void): void => {
    void listen<BoardPairingEvent>('board-pairing', (event) => callback(event.payload))
  }
}
