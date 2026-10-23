import { useBoards } from '@renderer/composables/useBoards'
import { useControls } from '@renderer/composables/useControls'
import { getPattern } from '@renderer/composables/useIntifacePatterns'
import { useLockedControls } from '@renderer/composables/useLockedControls'
import { useViewerGroups } from '@renderer/composables/useViewerGroups'
import type { Automation } from '@renderer/lib/tauri-bridge'
import { api } from '@renderer/lib/tauri-bridge'
import type { DescribeContext } from '@renderer/utils/automations'
import { controlsByAddress } from '@renderer/utils/controlAddresses'
import type { ComputedRef, Ref } from 'vue'
import { computed, ref, toRaw } from 'vue'

// Automations run in the backend, which sends the whole list on every change; this mirrors it.
const automations = ref<Automation[]>([])
// Ids of the automations currently driving something, for the "Running" chips.
// The backend says when each one starts and stops, pulses and delayed steps included.
const running = ref<Set<string>>(new Set())

// When each one last ran and how often, kept on this computer only (a convenience, not a log).
export interface AutomationRuns {
  at: number
  count: number
  // The last step it skipped, and why ("Skipped Toy: Intiface toys is off").
  skipped?: string
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
// Rust says it once per automation and session.
api.onAutomationSkipped(({ automationId, message }) => {
  const run = runs.value[automationId] ?? { at: 0, count: 0 }
  runs.value = { ...runs.value, [automationId]: { ...run, skipped: message } }
})

// A copy to start from: a new id, "copy" in its name, and off until you turn it on.
const copyOf = (automation: Automation, name = `${automation.name} copy`): Automation => ({
  ...structuredClone(toRaw(automation)),
  id: crypto.randomUUID(),
  name,
  enabled: false,
  createdBy: undefined
})

// Pasted JSON: one automation or a list. Each is saved as a new one, turned off; Rust checks it.
// Returns how many were saved, or throws with what went wrong.
const importJson = async (text: string): Promise<number> => {
  const parsed: unknown = JSON.parse(text)
  const list = (Array.isArray(parsed) ? parsed : [parsed]) as Automation[]
  for (const automation of list) {
    if (!automation || typeof automation !== 'object' || !automation.when || !Array.isArray(automation.then)) {
      throw new Error("That doesn't look like an automation.")
    }
    await api.automationSave(copyOf({ ...automation, onlyIf: automation.onlyIf ?? [] }, automation.name))
  }
  return list.length
}

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
  duplicate: (automation: Automation) => Promise<void>
  importJson: (text: string) => Promise<number>
} {
  const { boards } = useBoards()
  const { controls } = useControls()
  const { getLockedControlGroup } = useLockedControls()
  const { getGroup } = useViewerGroups()
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
      automationName: (id) => automations.value.find((a) => a.id === id)?.name,
      groupName: (id) => getGroup(id)?.name
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
    test: (id) => api.automationTest(id),
    duplicate: (automation) => api.automationSave(copyOf(automation)),
    importJson
  }
}
