import type { Addon } from '@renderer/composables/useAddons'
import { useAddons } from '@renderer/composables/useAddons'
import { useAreYouSureModal } from '@renderer/composables/useAreYouSureModal'
import { useAutomations } from '@renderer/composables/useAutomations'
import { addonUsage, savedControlTypes, usageMessage } from '@renderer/utils/addonUsage'

const names = (addons: Addon[]): string => {
  const list = addons.map((addon) => addon.name)

  return list.length > 1 ? `${list.slice(0, -1).join(', ')} and ${list.at(-1)}` : (list[0] ?? '')
}

// Turning an add-on on or off from anywhere in the app, with the questions that come with it:
// - on, when it needs an add-on that's off: "Avatar haptics needs Intiface toys. Turn both on?"
// - off, when another add-on that's on needs it: turn both off?
// - off, when controls or automations use it: how many, and that they're kept.
// Must be called from a component's setup (the confirmation is an overlay).
export function useAddonSwitch(): {
  // Resolves to whether it happened (false: cancelled).
  setAddon: (addon: Addon, enabled: boolean) => Promise<boolean>
} {
  const { addons, setEnabled } = useAddons()
  const { automations } = useAutomations()
  const { openModal: confirm } = useAreYouSureModal()
  const toast = useToast()

  const byId = (id: string): Addon | undefined => addons.value.find((a) => a.id === id)

  const turnOn = async (addon: Addon): Promise<boolean> => {
    const missing = addon.needs.map(byId).filter((a): a is Addon => !!a && !a.enabled)
    // Needs one of several and has none: the first one is offered.
    const anyOf = addon.needsAny.map(byId).filter((a): a is Addon => !!a)

    if (anyOf.length && !anyOf.some((a) => a.enabled)) missing.push(anyOf[0]!)

    if (
      missing.length &&
      !(await confirm({
        title: `Turn on ${names(missing)} too?`,
        message: `**${addon.name}** needs **${names(missing)}**. Turn both on?`,
        confirmText: 'Turn both on',
        danger: false
      }))
    ) {
      return false
    }

    for (const need of missing) await setEnabled(need.id, true)
    await setEnabled(addon.id, true)

    return true
  }

  const turnOff = async (addon: Addon): Promise<boolean> => {
    // Those that need it, or need one of several and it's the only one of them on.
    const dependents = addons.value.filter(
      (a) =>
        a.enabled &&
        (a.needs.includes(addon.id) || (a.needsAny.includes(addon.id) && !a.needsAny.some((id) => id !== addon.id && byId(id)?.enabled)))
    )
    const affected = [...dependents, addon]
    const controlTypes = savedControlTypes()
    const usage = affected
      .map((a) => usageMessage(a.name, addonUsage(a, controlTypes, automations.value)))
      .filter((message): message is string => !!message)

    if (dependents.length || usage.length) {
      const ok = await confirm({
        title: `Turn off ${names(affected)}?`,
        message: [
          dependents.length ? `**${names(dependents)}** ${dependents.length === 1 ? 'needs' : 'need'} **${addon.name}**, so ${dependents.length === 1 ? 'it' : 'they'} will turn off too.` : '',
          ...usage,
          'Devices stop first.'
        ]
          .filter(Boolean)
          .join(' '),
        confirmText: 'Turn off',
        danger: true
      })

      if (!ok) return false
    }

    for (const dependent of dependents) await setEnabled(dependent.id, false)
    await setEnabled(addon.id, false)

    return true
  }

  const setAddon = async (addon: Addon, enabled: boolean): Promise<boolean> => {
    try {
      return await (enabled ? turnOn(addon) : turnOff(addon))
    } catch (error) {
      toast.add({ title: `Could not turn ${enabled ? 'on' : 'off'} ${addon.name}`, description: String(error), color: 'error' })

      return false
    }
  }

  return { setAddon }
}
