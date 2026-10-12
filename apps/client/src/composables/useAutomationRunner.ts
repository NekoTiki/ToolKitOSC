import { useAvatarDetails } from '@renderer/composables/useAvatarDetails'
import { useChatbox } from '@renderer/composables/useChatbox'
import { useClientsDb } from '@renderer/composables/useClientsDb'
import { useControls } from '@renderer/composables/useControls'
import { useLockedControls } from '@renderer/composables/useLockedControls'
import { useOscMessages } from '@renderer/composables/useOscMessages'
import { useStopEverything } from '@renderer/composables/useStopEverything'
import type { AutomationRun, ControlAction, ProfileAction } from '@renderer/lib/tauri-bridge'
import { api } from '@renderer/lib/tauri-bridge'
import type { CommandWithoutIds, ControlType } from '@toolkitosc/shared-ui'
import { useIntifaceControl, useIntifacePatternControl } from '@toolkitosc/shared-ui'
import { watch } from 'vue'

// Carries out the automation steps that live in the app: controls, profiles, the chatbox and
// Stop everything, and tells the backend what automations can start from or check: viewers coming
// and going, controls resuming, the active profile. The backend decides when (src-tauri/src/automations/engine.rs) and sends
// `automation-run`; a hold comes as `start`, then `stop`, and whatever `start` changed is put
// back on `stop`. Automations act as you, so profile locks and limits don't apply to them.
// Stop everything still does: nothing new starts while it's on.

type Value = boolean | number | string

// Follows send at most this often, like the board follows.
const FOLLOW_MIN_GAP_MS = 50

let started = false

export function useAutomationRunner(): void {
  if (started) return
  started = true

  const { getControl, handleCommand } = useControls()
  const { currentLockedControlsGroup, lockedControlGroups } = useLockedControls()
  const { get } = useOscMessages()
  const { announceNotice } = useChatbox()
  const { paused, stop: stopEverything } = useStopEverything()
  const { controlValue: toyValues } = useIntifaceControl()
  const { controlValue: patternValues } = useIntifacePatternControl()
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

  // What each running hold changed, keyed by automation and step, to put back on `stop`.
  const held = new Map<string, Value | string | null>()
  const pulses = new Map<string, ReturnType<typeof setTimeout>>()
  const follows = new Map<
    string,
    { sentAt: number; timer?: ReturnType<typeof setTimeout>; value: number }
  >()

  // Holds and pulses belong to the old avatar's controls and profiles: forget them.
  useAvatarDetails(() => {
    held.clear()
    pulses.forEach((timer) => clearTimeout(timer))
    pulses.clear()
    follows.forEach((follow) => clearTimeout(follow.timer))
    follows.clear()
  })

  const current = (control: ControlType): Value => {
    switch (control.type) {
      case 'boolean':
        return !!get<Value>(control.inputAddress, false)
      case 'boolean-group':
        return !!get<Value>(control.inputs[0]?.inputAddress ?? '', false)
      case 'enum':
      case 'step-enum': {
        const value = get<Value | null>(control.inputAddress, null)
        return Math.max(
          0,
          control.options.findIndex((option) => option.value === Number(value))
        )
      }
      case 'slider':
        return Number(get<Value>(control.inputAddress, 0))
      case 'intiface-toy':
        return toyValues.value.get(control.id) ?? 0
      case 'intiface-pattern':
        return patternValues.value.get(control.id) ?? 'off'
      default:
        return 0
    }
  }

  const send = (control: ControlType, value?: Value): void => {
    handleCommand(
      control,
      (value === undefined
        ? { type: control.type }
        : { type: control.type, value }) as CommandWithoutIds
    )
  }

  const follow = (key: string, control: ControlType, value: number): void => {
    const state = follows.get(key) ?? { sentAt: 0, value }
    state.value = value
    follows.set(key, state)
    if (state.timer) return

    const wait = state.sentAt + FOLLOW_MIN_GAP_MS - Date.now()
    if (wait <= 0) {
      state.sentAt = Date.now()
      send(control, value)
      return
    }
    state.timer = setTimeout(() => {
      state.timer = undefined
      state.sentAt = Date.now()
      if (!paused.value) send(control, state.value)
    }, wait)
  }

  const runControl = (key: string, action: ControlAction, run: AutomationRun): void => {
    const control = getControl(action.groupId, action.controlId)
    if (!control || control.unavailable) return

    if (run.phase === 'stop') {
      if (!held.has(key)) return
      const before = held.get(key) as Value
      held.delete(key)
      // Stop everything already switched toys and patterns off; don't start them again.
      if (paused.value && (control.type === 'intiface-toy' || control.type === 'intiface-pattern'))
        return
      send(control, before)
      return
    }

    const step = action.run
    if (run.phase === 'follow') {
      if (step.type === 'follow') follow(key, control, step.min + run.value * (step.max - step.min))
      return
    }

    switch (step.type) {
      case 'set':
        send(control, step.value)
        break
      case 'toggle':
        send(control, !current(control))
        break
      case 'press':
        send(control)
        break
      case 'pulse': {
        clearTimeout(pulses.get(key))
        send(control, step.value)
        pulses.set(
          key,
          setTimeout(() => {
            pulses.delete(key)
            if (!paused.value) send(control, 0)
          }, step.ms)
        )
        break
      }
      case 'hold':
        if (run.phase !== 'start' && run.phase !== 'run') break
        if (!held.has(key)) held.set(key, current(control))
        send(control, step.value)
        break
    }
  }

  const runProfile = (key: string, action: ProfileAction, run: AutomationRun): void => {
    // A profile that was deleted since is skipped.
    if (action.profileId && !lockedControlGroups.value.some((p) => p.id === action.profileId))
      return

    if (run.phase === 'stop') {
      if (!held.has(key)) return
      const before = held.get(key) as string | null
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
      case 'control':
        runControl(key, run.action, run)
        break
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
