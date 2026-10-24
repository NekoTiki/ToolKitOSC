import { useAutomations } from '@renderer/composables/useAutomations'
import { useChatbox } from '@renderer/composables/useChatbox'
import { useClientsDb } from '@renderer/composables/useClientsDb'
import { useStopEverything } from '@renderer/composables/useStopEverything'
import { api } from '@renderer/lib/tauri-bridge'
import { watch } from 'vue'

// Carries out the one automation step that lives in the app, the chatbox (Control, profile and
// Stop everything steps run in Rust, see src-tauri/src/automations/), and tells the backend what
// automations can start from or check: viewers coming and going. Stop everything still applies:
// nothing new is said while it's on.

let started = false

export function useAutomationRunner(): void {
  if (started) return
  started = true

  const { announceNotice } = useChatbox()
  const { paused } = useStopEverything()
  const { onlineClients } = useClientsDb()
  const { automations } = useAutomations()
  const toast = useToast()

  // A step of an add-on that's off was skipped. Rust says so once per automation.
  api.onAutomationSkipped(({ message }) => toast.add({ title: 'Automation step skipped', description: message, icon: 'i-lucide-puzzle', color: 'warning' }))

  // Before the events below, so "only if viewers are on" already counts a first viewer.
  watch(
    () => onlineClients.value.size,
    (viewers) => void api.automationsSetContext({ viewers }),
    { immediate: true }
  )

  watch(
    () => onlineClients.value.size,
    (now, before) => {
      if (now > before) {
        if (before === 0) void api.automationEvent({ type: 'first-viewer' })
        void api.automationEvent({ type: 'viewer-joined' })
      } else if (now === 0 && before > 0) {
        void api.automationEvent({ type: 'last-viewer-left' })
      }
    }
  )

  api.onAutomationChatbox((text) => {
    if (!paused.value) announceNotice(text)
  })

  // Automations that run when Stop everything turns on: their chatbox steps still post.
  const onStop = (id: string): boolean =>
    automations.value.some((a) => a.id === id && a.when.kind === 'app-event' && a.when.event.type === 'stopped')

  api.onAutomationRun((run) => {
    if ((paused.value && !onStop(run.automationId)) || run.phase !== 'run') return

    if (run.action.kind === 'chatbox') announceNotice(run.action.text)
  })
}
