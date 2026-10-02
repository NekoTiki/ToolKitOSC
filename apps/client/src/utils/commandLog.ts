import type { Command } from '@renderer/db/commands.db'
import type { OpenShockCommandResult } from '@vrc-osc-toolkit/shared-ui'

// Shared by every place that renders logged Command rows (the Activity and Viewers pages, the
// Controls page's log line), however they're filtered.
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
