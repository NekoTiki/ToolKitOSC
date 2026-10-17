import type { AddonManifest, Automation } from '@renderer/lib/tauri-bridge'

export interface AddonUsage {
  controls: number
  automations: number
}

// Every avatar's saved controls, from localStorage (`controls_<avatarId>`, see useControls.ts),
// not only the worn avatar's: turning an add-on off affects them all.
export const savedControlTypes = (): string[] => {
  const types: string[] = []

  for (let i = 0; i < localStorage.length; i++) {
    const key = localStorage.key(i)

    if (!key?.startsWith('controls_avtr_')) continue

    try {
      const groups: unknown = JSON.parse(localStorage.getItem(key) ?? '[]')

      if (!Array.isArray(groups)) continue
      groups.forEach((group: { controls?: { type?: string }[] }) => group.controls?.forEach((control) => control.type && types.push(control.type)))
    } catch {
      // Not ours, or damaged: nothing to count.
    }
  }

  return types
}

// How many controls and automations use this add-on, for the turn-off confirmation.
export const addonUsage = (addon: AddonManifest, controlTypes: string[], automations: Automation[]): AddonUsage => ({
  controls: controlTypes.filter((type) => addon.controlTypes.includes(type)).length,
  automations: automations.filter(
    (automation) =>
      automation.then.some(
        (step) => addon.automationKinds.includes(step.kind) || (step.kind === 'control' && addon.controlTypes.includes(step.controlType))
      ) ||
      (automation.when.kind === 'app-event' &&
        automation.when.event.type === 'viewer-command' &&
        !!automation.when.event.controlType &&
        addon.controlTypes.includes(automation.when.event.controlType))
  ).length
})

const plural = (n: number, word: string): string => `${n} ${word}${n === 1 ? '' : 's'}`

// "3 controls and 1 automation use OpenShock. They'll be kept and greyed out." Null when nothing
// uses it.
export const usageMessage = (name: string, usage: AddonUsage): string | null => {
  const parts = [usage.controls && plural(usage.controls, 'control'), usage.automations && plural(usage.automations, 'automation')].filter(
    (part): part is string => !!part
  )

  if (!parts.length) return null

  const total = usage.controls + usage.automations

  return `${parts.join(' and ')} ${total === 1 ? 'uses' : 'use'} ${name}. ${total === 1 ? "It'll" : "They'll"} be kept and greyed out until you turn it back on.`
}
