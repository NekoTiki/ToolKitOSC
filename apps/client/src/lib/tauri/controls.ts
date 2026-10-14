// Running a control as you, without limits - src-tauri/src/controls.rs.
import { invoke } from '@tauri-apps/api/core'

export const controlsApi = {
  // Runs a command on a control as you, without limits.
  controlRun: (control: unknown, command: unknown, viewerName?: string): Promise<{ shock: unknown; patternRun: number | null }> =>
    invoke('control_run', { control, command, viewerName: viewerName ?? null })
}
