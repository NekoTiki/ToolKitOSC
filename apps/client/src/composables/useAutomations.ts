import { useBoards } from '@renderer/composables/useBoards'
import { useControls } from '@renderer/composables/useControls'
import { getPattern } from '@renderer/composables/useIntifacePatterns'
import { useLockedControls } from '@renderer/composables/useLockedControls'
import type { Automation } from '@renderer/lib/tauri-bridge'
import { api } from '@renderer/lib/tauri-bridge'
import type { DescribeContext } from '@renderer/utils/automations'
import { controlsByAddress } from '@renderer/utils/controlAddresses'
import type { ComputedRef, Ref } from 'vue'
import { computed, ref } from 'vue'

// Automations run in the backend, which sends the whole list on every change; this mirrors it.
const automations = ref<Automation[]>([])
// Ids of the automations currently driving something, for the "Running" chips.
// The backend says when each one starts and stops, pulses and delayed steps included.
const running = ref<Set<string>>(new Set())

// When each one last ran and how often, kept on this computer only (a convenience, not a log).
export interface AutomationRuns {
  at: number
  count: number
}
const RUNS_KEY = 'automations_runs'
const loadRuns = (): Record<string, AutomationRuns> => {
  try {
    return JSON.parse(localStorage.getItem(RUNS_KEY) ?? '{}') as Record<string, AutomationRuns>
  } catch {
    return {}
  }
}
const runs = ref<Record<string, AutomationRuns>>(loadRuns())

api.onAutomationRan((id) => {
  runs.value = { ...runs.value, [id]: { at: Date.now(), count: (runs.value[id]?.count ?? 0) + 1 } }
  try {
    localStorage.setItem(RUNS_KEY, JSON.stringify(runs.value))
  } catch {
    // Storage full or blocked: the counts just don't survive a restart.
  }
})
// Module scope, like useBoards.ts, so nothing is missed while a page mounts.
api.onAutomationsChanged((value) => (automations.value = value))
api.onAutomationActivity(({ id, running: isRunning }) => {
  const next = new Set(running.value)
  if (isRunning) next.add(id)
  else next.delete(id)
  running.value = next
})
void api.automationsList().then((value) => (automations.value = value))

export function useAutomations(): {
  automations: Ref<Automation[]>
  running: Ref<Set<string>>
  runs: Ref<Record<string, AutomationRuns>>
  // Board, control and profile names for describe(). Controls and profiles are the worn avatar's.
  describeContext: ComputedRef<DescribeContext>
  // Rejects with a message to show.
  save: (automation: Automation) => Promise<void>
  remove: (id: string) => Promise<void>
  setEnabled: (id: string, enabled: boolean) => Promise<void>
  test: (id: string) => Promise<void>
} {
  const { boards } = useBoards()
  const { controls } = useControls()
  const { getLockedControlGroup } = useLockedControls()
  const describeContext = computed((): DescribeContext => {
    const all = controls.value.flatMap((group) => group.controls)
    const byAddress = controlsByAddress(all)
    const control = (controlId: string) => all.find((c) => c.id === controlId)
    return {
      boards: boards.value,
      controlName: (controlId) => control(controlId)?.name,
      profileName: (id) => getLockedControlGroup(id)?.name,
      control,
      controlForParameter: (address) => byAddress.get(address),
      patternName: (id) => getPattern(id)?.name,
      automationName: (id) => automations.value.find((a) => a.id === id)?.name
    }
  })

  return {
    automations,
    running,
    runs,
    describeContext,
    save: (automation) => api.automationSave(automation),
    remove: (id) => api.automationDelete(id),
    setEnabled: (id, enabled) => api.automationSetEnabled(id, enabled),
    test: (id) => api.automationTest(id)
  }
}
