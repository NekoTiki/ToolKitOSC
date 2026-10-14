import { useAvatarDetails } from '@renderer/composables/useAvatarDetails'
import { useChatbox } from '@renderer/composables/useChatbox'
import { useClientsDb } from '@renderer/composables/useClientsDb'
import { useLockedControls } from '@renderer/composables/useLockedControls'
import { useStopEverything } from '@renderer/composables/useStopEverything'
import type { AutomationRun, ProfileAction } from '@renderer/lib/tauri-bridge'
import { api } from '@renderer/lib/tauri-bridge'
import { watch } from 'vue'

// Carries out the automation steps that live in the app: profiles, the chatbox and Stop
// everything (Control steps run in Rust, see src-tauri/src/automations/control_runs.rs), and tells
// the backend what automations can start from or check: viewers coming and going, controls
// resuming, the active profile. The backend decides when (src-tauri/src/automations/engine.rs) and
// sends `automation-run`; a hold comes as `start`, then `stop`, and whatever `start` changed is put
// back on `stop`. Stop everything still applies: nothing new starts while it's on.

let started = false

export function useAutomationRunner(): void {
  if (started) return
  started = true

  const { currentLockedControlsGroup, lockedControlGroups } = useLockedControls()
  const { announceNotice } = useChatbox()
  const { paused, stop: stopEverything } = useStopEverything()
  const { onlineClients } = useClientsDb()

  // Before the events below, so "only if viewers are on" already counts a first viewer.
  watch(
    [currentLockedControlsGroup, () => onlineClients.value.size],
    ([profileId, viewers]) => void api.automationsSetContext({ profileId, viewers }),
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

  // After useStopEverythingTriggers has told the backend it's no longer paused (it's set up first).
  watch(paused, (now, before) => {
    if (before && !now) void api.automationEvent({ type: 'resumed' })
  })

  // What each running profile hold changed, keyed by automation and step, to put back on `stop`.
  const held = new Map<string, string | null>()

  // Holds belong to the old avatar's profiles: forget them.
  useAvatarDetails(() => held.clear())

  const runProfile = (key: string, action: ProfileAction, run: AutomationRun): void => {
    // A profile that was deleted since is skipped.
    if (action.profileId && !lockedControlGroups.value.some((p) => p.id === action.profileId))
      return

    if (run.phase === 'stop') {
      if (!held.has(key)) return
      const before = held.get(key) ?? null
      held.delete(key)
      // Only if nothing else switched it in the meantime.
      if (currentLockedControlsGroup.value === action.profileId)
        currentLockedControlsGroup.value = before
      return
    }
    if (run.phase === 'start' && !held.has(key)) held.set(key, currentLockedControlsGroup.value)
    currentLockedControlsGroup.value = action.profileId
  }

  api.onAutomationChatbox((text) => {
    if (!paused.value) announceNotice(text)
  })

  api.onAutomationRun((run) => {
    const key = `${run.automationId}:${run.step}`
    // Nothing new starts while Stop everything is on; holds may still let go.
    if (paused.value && run.phase !== 'stop') return

    switch (run.action.kind) {
      case 'profile':
        runProfile(key, run.action, run)
        break
      case 'chatbox':
        if (run.phase === 'run') announceNotice(run.action.text)
        break
      case 'stop-everything':
        if (run.phase === 'run') stopEverything()
        break
    }
  })
}
