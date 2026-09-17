import type { Command } from '@renderer/db/commands.db'
import type { OpenShockCommandResult } from '@vrc-osc-toolkit/shared-ui'

// Shared between ClientLogsModal.vue (a client's whole history) and ControlLogsModal.vue (one
// control's whole history) - both render the exact same kind of Command row, just filtered
// differently (by ip/discordId vs. by controlId).
export const isOpenShockCommandValue = (
  value: Command['value']
): value is OpenShockCommandResult => typeof value === 'object' && value !== null && 'shockers' in value

export const formatCommandValue = (value: Command['value']): string => {
  if (isOpenShockCommandValue(value)) {
    return `${value.intensity}% • ${(value.duration / 1000).toFixed(1)}s`
  }
  if (typeof value === 'boolean') return value ? 'On' : 'Off'
  if (typeof value === 'number') return Number.isInteger(value) ? String(value) : value.toFixed(2)
  if (value === undefined) return '-'

  return String(value)
}
