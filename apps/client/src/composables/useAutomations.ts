import type { Automation } from '@renderer/lib/tauri-bridge'
import { api } from '@renderer/lib/tauri-bridge'
import type { Ref } from 'vue'
import { ref } from 'vue'

// Automations run in the backend, which sends the whole list on every change; this mirrors it.
const automations = ref<Automation[]>([])
// Ids of the automations currently driving something, for the "Running" chips.
const running = ref<Set<string>>(new Set())
const timers = new Map<string, ReturnType<typeof setTimeout>>()

const setRunning = (id: string, value: boolean): void => {
  const next = new Set(running.value)
  if (value) next.add(id)
  else next.delete(id)
  running.value = next
}

// Module scope, like useBoards.ts, so nothing is missed while a page mounts.
api.onAutomationsChanged((value) => (automations.value = value))
api.onAutomationActivity(({ id, running: isRunning, forMs }) => {
  clearTimeout(timers.get(id))
  timers.delete(id)
  setRunning(id, isRunning)

  // Pulses and toggles end on the board without another event.
  if (isRunning && forMs) {
    timers.set(
      id,
      setTimeout(() => setRunning(id, false), forMs)
    )
  }
})
void api.automationsList().then((value) => (automations.value = value))

export function useAutomations(): {
  automations: Ref<Automation[]>
  running: Ref<Set<string>>
  // Rejects with a message to show.
  save: (automation: Automation) => Promise<void>
  remove: (id: string) => Promise<void>
  setEnabled: (id: string, enabled: boolean) => Promise<void>
  test: (id: string) => Promise<void>
} {
  return {
    automations,
    running,
    save: (automation) => api.automationSave(automation),
    remove: (id) => api.automationDelete(id),
    setEnabled: (id, enabled) => api.automationSetEnabled(id, enabled),
    test: (id) => api.automationTest(id)
  }
}
