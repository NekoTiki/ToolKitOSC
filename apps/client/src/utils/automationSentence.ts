import type {
  AppEvent,
  Automation,
  AutomationGuard,
  AutomationStep,
  AutomationTrigger,
  ControlAction,
  OutputAction,
  ParameterValueType
} from '@renderer/lib/tauri-bridge'
import type { DescribeContext } from '@renderer/utils/automations'
import { CONDITIONS, parameterName, triggerCondition } from '@renderer/utils/automations'
import type { ControlType } from '@toolkitosc/shared-ui'
import { BHAPTICS_EFFECTS, BHAPTICS_LAYOUT, OWO_SENSATIONS } from '@toolkitosc/shared-ui'

// An automation as one plain sentence ("When Boop turns on, buzz Lush at 60% for 2 s, any time,
// every time."), for the list and the Easy editor. Names and values are `strong`, so the page can
// bold them. describe() in automations.ts stays the short technical form (assistants, approvals).

export type Rich = { text: string; strong?: boolean }[]

export interface Sentence {
  // "While" for a state ("while Boop is on"), "When" for a moment or a follow.
  lead: 'When' | 'While'
  when: Rich
  steps: { delayMs: number; text: Rich; missing: boolean }[]
  only: Rich | null
  often: Rich
}

// The choices Easy offers. Anything else is an Advanced setting (see easyExtras).
export const EASY_DELAYS = [0, 1000, 3000, 10_000]
export const EASY_PULSES = [1000, 2000, 5000, 10_000]
export const EASY_HELD = [0, 500, 1000, 3000]
export const EASY_COOLDOWNS = [0, 5000, 60_000]

const strong = (text: string): Rich[number] => ({ text, strong: true })

// rich`buzz ${strong(name)} at ${strong('60%')}`: plain text with bold parts.
const rich = (strings: TemplateStringsArray, ...values: (string | Rich[number])[]): Rich =>
  strings.flatMap((text, i) => {
    const value = values[i]
    const parts: Rich = text ? [{ text }] : []
    if (value !== undefined) parts.push(typeof value === 'string' ? { text: value } : value)
    return parts
  })

const pct = (value: number): string => `${Math.round(value * 100)}%`

// 2 s, 1.75 s, 1 min.
export const seconds = (ms: number): string =>
  ms >= 60_000 && ms % 60_000 === 0 ? `${ms / 60_000} min` : `${Math.round(ms / 10) / 100} s`

const parameterLabel = (address: string, context?: DescribeContext): string =>
  context?.controlForParameter?.(address)?.name ?? parameterName(address)

// An Int's value by the option name of the control that drives it, when there is one.
const parameterValue = (address: string, valueType: ParameterValueType, value: number, context?: DescribeContext): string => {
  if (valueType === 'Bool') return value ? 'on' : 'off'
  if (valueType === 'Float') return pct(value)
  const control = context?.controlForParameter?.(address)
  const option = control?.type === 'enum' || control?.type === 'step-enum' ? control.options.find((o) => o.value === value) : undefined
  return option?.name ?? String(value)
}

const describeEvent = (event: AppEvent, context?: DescribeContext): Rich => {
  switch (event.type) {
    case 'viewer-command':
      if (event.controlId) return rich`a viewer uses ${strong(context?.controlName?.(event.controlId) ?? 'a control')}`
      return event.controlType === 'open-shock-shocker' ? rich`a viewer sends a shock` : rich`a viewer uses a control`
    case 'viewer-joined':
      return rich`a viewer joins`
    case 'first-viewer':
      return rich`the first viewer joins`
    case 'last-viewer-left':
      return rich`the last viewer leaves`
    case 'resumed':
      return rich`you turn ${strong('Stop everything')} off`
  }
}

export const sentenceWhen = (when: AutomationTrigger, context?: DescribeContext): Rich => {
  switch (when.kind) {
    case 'app-event':
      return describeEvent(when.event, context)
    case 'leash':
      return when.event === 'grabbed'
        ? rich`someone grabs your leash`
        : when.event === 'running'
          ? rich`your leash is pulled to running`
          : rich`your leash is let go`
    case 'avatar-changed':
      return when.avatarId ? rich`you put on ${strong(when.avatarName ?? 'an avatar')}` : rich`you switch avatar`
    case 'avatar-parameter': {
      if (!when.parameter) return rich`something on your avatar changes`
      const name = strong(parameterLabel(when.parameter, context))
      const { condition } = when
      const value = 'value' in condition ? strong(parameterValue(when.parameter, when.valueType, condition.value, context)) : ''
      const text = {
        'turns-on': rich`${name} turns on`,
        'turns-off': rich`${name} turns off`,
        'is-on': rich`${name} is on`,
        becomes: rich`${name} becomes ${value}`,
        is: rich`${name} is ${value}`,
        'rises-above': rich`${name} goes past ${value}`,
        'is-above': rich`${name} is above ${value}`,
        any: rich`${name} changes`
      }[condition.type]
      if (when.heldMs) text.push(...rich` for at least ${strong(seconds(when.heldMs))}`)
      if (when.anyAvatar) text.push(...rich` on any avatar`)
      return text
    }
  }
}

// "Set" value of a control, in its own words: an option's name, on/off, a percentage.
const controlValue = (control: ControlType | undefined, value: boolean | number | string): string => {
  if (typeof value === 'boolean') return value ? 'on' : 'off'
  switch (control?.type) {
    case 'enum':
    case 'step-enum':
      return control.options[Number(value)]?.name ?? String(value)
    case 'boolean-enum':
      return control.inputs[Number(value)]?.name ?? String(value)
    case 'intiface-pattern':
      return control.allowedPatterns.find((p) => p.id === value)?.name ?? 'a pattern'
    default:
      return typeof value === 'number' ? pct(value) : String(value)
  }
}

const range = (min: number, max: number): Rich => (min === 0 && max === 1 ? [] : rich`, from ${strong(pct(min))} to ${strong(pct(max))}`)

const controlStep = (step: ControlAction, context?: DescribeContext): Rich => {
  const control = context?.control?.(step.controlId)
  const name = strong(control?.name ?? context?.controlName?.(step.controlId) ?? 'a control')
  const toy = step.controlType === 'intiface-toy'
  const { run } = step
  switch (run.type) {
    case 'pulse':
      return rich`buzz ${name} at ${strong(pct(run.value))} for ${strong(seconds(run.ms))}`
    case 'toggle':
      return rich`flip ${name}`
    case 'press':
      if (control?.type === 'open-shock-shocker') return control.mode === 'Vibrate' ? rich`vibrate with ${name}` : rich`shock with ${name}`
      return step.controlType === 'preset' ? rich`apply ${name}` : rich`press ${name}`
    case 'follow':
      return [...(toy ? rich`buzz ${name} following it` : rich`move ${name} with it`), ...range(run.min, run.max)]
    case 'set':
    case 'hold': {
      const value = strong(controlValue(control, run.value))
      const text =
        typeof run.value === 'boolean'
          ? rich`turn ${name} ${value}`
          : toy
            ? rich`buzz ${name} at ${value}`
            : step.controlType === 'intiface-pattern'
              ? run.value === 'off' ? rich`stop ${name}` : rich`play ${value} on ${name}`
              : rich`set ${name} to ${value}`
      return run.type === 'hold' ? [...text, ...rich` for as long as it lasts`] : text
    }
  }
}

const boardOutput = (output: OutputAction, name: Rich[number]): Rich => {
  switch (output.type) {
    case 'pulse':
      return [...rich`switch on ${name} for ${strong(seconds(output.onMs))}`, ...(output.count > 1 ? rich`, ${strong(`${output.count} times`)}` : [])]
    case 'toggle':
      return rich`flip ${name}`
    case 'off':
      return rich`switch off ${name}`
    case 'hold':
      return rich`switch on ${name} for as long as it lasts`
    case 'follow':
      return [...rich`dim ${name} with it`, ...range(output.min, output.max)]
  }
}

export const sentenceStep = (step: AutomationStep, context?: DescribeContext): Rich => {
  switch (step.kind) {
    case 'control':
      return controlStep(step, context)
    case 'avatar-parameter': {
      const name = strong(step.parameter ? parameterLabel(step.parameter, context) : 'a parameter')
      const { set } = step
      if (set.type === 'toggle') return rich`flip ${name}`
      if (set.type === 'follow') return [...rich`move ${name} with it`, ...range(set.min, set.max)]
      const value = strong(parameterValue(step.parameter, step.valueType, set.value, context))
      const text = step.valueType === 'Bool' ? rich`turn ${name} ${value}` : rich`set ${name} to ${value}`
      return set.type === 'hold' ? [...text, ...rich` for as long as it lasts`] : text
    }
    case 'board-output': {
      const board = context?.boards.find((b) => b.id === step.boardId)
      const name = strong(board?.config?.find((o) => o.pin === step.pin)?.label ?? (step.pin < 0 ? 'an output' : `GPIO ${step.pin}`))
      const text = boardOutput(step.output, name)
      return step.chatbox ? [...text, ...rich` and say so in the chatbox`] : text
    }
    case 'profile': {
      const text = step.profileId
        ? rich`switch to the ${strong(context?.profileName?.(step.profileId) ?? 'a')} profile`
        : rich`switch to ${strong('No profile')}`
      return step.whileHeld ? [...text, ...rich` for as long as it lasts`] : text
    }
    case 'chatbox':
      return rich`say ${strong(`“${step.text}”`)} in the chatbox`
    case 'stop-everything':
      return [strong('stop everything')]
    case 'leash-enabled':
      return rich`turn ${strong('Leash moves me')} ${strong(step.enabled ? 'on' : 'off')}`
    case 'bhaptics-pulse':
      return rich`buzz your bHaptics ${strong(BHAPTICS_LAYOUT[step.position].label.toLowerCase())} at ${strong(pct(step.intensity))} for ${strong(seconds(step.ms))}`
    case 'bhaptics-effect':
      return rich`play ${strong(BHAPTICS_EFFECTS.find((e) => e.id === step.effect)?.label ?? 'an effect')} on your bHaptics gear`
    case 'owo-sensation':
      return rich`play ${strong(OWO_SENSATIONS.find((s) => s.id === step.sensation)?.label ?? 'a sensation')} on your OWO Skin at ${strong(pct(step.intensity))}`
  }
}

// A step that still needs its target or text, shown as a dashed slot.
export const stepMissing = (step: AutomationStep): boolean => {
  switch (step.kind) {
    case 'control':
      return !step.controlId
    case 'avatar-parameter':
      return !step.parameter
    case 'board-output':
      return step.pin < 0
    case 'chatbox':
      return !step.text.trim()
    default:
      return false
  }
}

const sentenceGuard = (guard: AutomationGuard, context?: DescribeContext): Rich => {
  switch (guard.kind) {
    case 'viewers':
      return guard.online ? rich`someone is watching` : rich`nobody is watching`
    case 'profile':
      return guard.profileId ? rich`the ${strong(context?.profileName?.(guard.profileId) ?? 'a')} profile is on` : rich`no profile is on`
    case 'parameter': {
      const name = strong(guard.parameter ? parameterLabel(guard.parameter, context) : 'a parameter')
      const { condition } = guard
      if (condition.type === 'is-on') return rich`${name} is ${strong(guard.not ? 'off' : 'on')}`
      const value = strong('value' in condition ? parameterValue(guard.parameter, guard.valueType, condition.value, context) : '')
      const verb = condition.type === 'is-above' ? (guard.not ? 'is not above' : 'is above') : guard.not ? 'is not' : 'is'
      return rich`${name} ${verb} ${value}`
    }
  }
}

export const sentenceOnly = (guards: AutomationGuard[], context?: DescribeContext): Rich | null =>
  guards.length ? [{ text: 'only when ' }, ...guards.flatMap((guard, i) => [...(i ? [{ text: ' and ' }] : []), ...sentenceGuard(guard, context)])] : null

export const sentenceOften = (cooldownMs: number): Rich => (cooldownMs ? rich`at most every ${strong(seconds(cooldownMs))}` : [strong('every time')])

export const sentence = (automation: Automation, context?: DescribeContext): Sentence => {
  const condition = triggerCondition(automation.when)
  return {
    lead: !CONDITIONS[condition].moment && condition !== 'any' ? 'While' : 'When',
    when: sentenceWhen(automation.when, context),
    steps: automation.then.map((step) => ({ delayMs: step.delayMs, text: sentenceStep(step, context), missing: stepMissing(step) })),
    only: sentenceOnly(automation.onlyIf, context),
    often: sentenceOften(automation.cooldownMs)
  }
}

// The whole sentence as plain text, for a tooltip or an aria-label.
export const plainSentence = (s: Sentence): string =>
  [
    `${s.lead} ${s.when.map((p) => p.text).join('')}`,
    ...s.steps.map((step, i) => `${i && step.delayMs ? `then after ${seconds(step.delayMs)} ` : i ? 'then ' : ''}${step.text.map((p) => p.text).join('')}`),
    s.only?.map((p) => p.text).join('') ?? 'any time',
    s.often.map((p) => p.text).join('')
  ].join(', ') + '.'

// What Easy mode can't show or change: these stay as they are, and Advanced edits them.
export const easyExtras = (automation: Automation): string[] => {
  const extras: string[] = []
  const { when } = automation
  if (when.kind === 'avatar-parameter') {
    if (when.anyAvatar) extras.push('Runs on any avatar with this parameter')
    if (!EASY_HELD.includes(when.heldMs)) extras.push(`Has to last ${seconds(when.heldMs)} first`)
  }
  automation.then.forEach((step, i) => {
    const n = i + 1
    if (!EASY_DELAYS.includes(step.delayMs)) extras.push(`Step ${n} waits ${seconds(step.delayMs)}`)
    if (step.kind === 'control') {
      if (step.run.type === 'pulse' && !EASY_PULSES.includes(step.run.ms)) extras.push(`Step ${n} lasts ${seconds(step.run.ms)}`)
      if (step.run.type === 'follow' && (step.run.min !== 0 || step.run.max !== 1)) extras.push(`Step ${n} follows within a range`)
    }
    if (step.kind === 'avatar-parameter' && step.set.type === 'follow' && (step.set.min !== 0 || step.set.max !== 1)) {
      extras.push(`Step ${n} follows within a range`)
    }
    if (step.kind === 'board-output') {
      const { output } = step
      if (output.type === 'pulse' && output.count > 1) extras.push(`Step ${n} repeats ${output.count} times`)
      if (output.type === 'pulse' && !EASY_PULSES.includes(output.onMs)) extras.push(`Step ${n} lasts ${seconds(output.onMs)}`)
      if (output.type === 'follow' && (output.min !== 0 || output.max !== 1)) extras.push(`Step ${n} dims within a range`)
    }
  })
  if (!EASY_COOLDOWNS.includes(automation.cooldownMs)) extras.push(`Waits ${seconds(automation.cooldownMs)} between runs`)
  return extras
}
