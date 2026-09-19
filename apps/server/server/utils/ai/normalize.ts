import { randomUUID } from 'node:crypto'

import type { ControlGroup, ControlType } from '@vrc-osc-toolkit/shared-ui'

import { ICON_CHOICES } from './prompt'
import type { AiControlSuggestion, AiParameterInput, AiSuggestionResult } from './types'

const MAX_GROUPS = 40
const MAX_CONTROLS_PER_GROUP = 60
const ICON_SET = new Set<string>(ICON_CHOICES)

// undefined (not a made-up fallback) for anything the model didn't pick from the closed list the
// prompt gave it - BaseControl.icon is already optional, so the control just renders without one.
const validIcon = (icon: unknown): string | undefined =>
  typeof icon === 'string' && ICON_SET.has(icon) ? icon : undefined

// Every Bool-kind address in `addresses` that's actually in this avatar's parameter list, in the
// model's given order - shared by 'boolean-group' (addresses) and 'boolean-enum'
// (trueAddresses/falseAddresses), which both only make sense over boolean parameters.
function validBoolAddresses(
  addresses: unknown,
  byAddress: Map<string, AiParameterInput>
): string[] {
  if (!Array.isArray(addresses)) return []

  return addresses.filter(
    (address): address is string =>
      typeof address === 'string' && byAddress.get(address)?.kind === 'Bool'
  )
}

function buildBooleanGroup(
  suggestion: AiControlSuggestion,
  byAddress: Map<string, AiParameterInput>
): ControlType | null {
  const addresses = validBoolAddresses(suggestion.addresses, byAddress)

  // A "group" of one is just a plain toggle - falls through to the normal boolean path instead
  // of a degenerate one-item boolean-group.
  const [singleAddress] = addresses

  if (addresses.length === 1 && singleAddress) {
    return {
      id: randomUUID(),
      name: suggestion.name,
      type: 'boolean',
      icon: validIcon(suggestion.icon),
      inputAddress: singleAddress
    }
  }

  if (addresses.length < 2) return null

  return {
    id: randomUUID(),
    name: suggestion.name,
    type: 'boolean-group',
    icon: validIcon(suggestion.icon),
    inputs: addresses.map((inputAddress) => ({ inputAddress }))
  }
}

function buildBooleanEnum(
  suggestion: AiControlSuggestion,
  byAddress: Map<string, AiParameterInput>
): ControlType | null {
  if (!Array.isArray(suggestion.options)) return null

  const inputs = suggestion.options
    .map((option) => {
      const trueAddresses = validBoolAddresses(option.trueAddresses, byAddress)

      // No verified "on" address at all means this option is pure hallucination - drop just this
      // option rather than the whole control, same "one bad item doesn't sink the rest" approach
      // as everywhere else here.
      if (!option.name || trueAddresses.length === 0) return null

      return {
        id: randomUUID(),
        name: option.name,
        icon: validIcon(option.icon),
        inputAddress: {
          true: trueAddresses,
          false: validBoolAddresses(option.falseAddresses, byAddress)
        }
      }
    })
    .filter((input): input is NonNullable<typeof input> => input !== null)

  // A "choice" needs at least two real alternatives - can't safely guess what a lone surviving
  // option should even mean, so the whole control is dropped rather than half-repaired.
  if (inputs.length < 2) return null

  return {
    id: randomUUID(),
    name: suggestion.name,
    type: 'boolean-enum',
    icon: validIcon(suggestion.icon),
    inputs
  }
}

// Turns a provider's raw suggestion into real ControlGroup[] - the safety net against a model
// that (despite the schema) hallucinates an address, gets a type/kind mismatch, or gives a
// 'step-enum'/'boolean-enum' too few options. Anything that doesn't check out is dropped rather
// than thrown on, since one bad control shouldn't sink an otherwise-good response; ids are always
// generated here, never trusted from the model.
export function normalizeSuggestion(
  result: AiSuggestionResult,
  parameters: AiParameterInput[]
): ControlGroup[] {
  const byAddress = new Map(parameters.map((p) => [p.address, p]))

  const groups: ControlGroup[] = []

  for (const group of result.groups.slice(0, MAX_GROUPS)) {
    if (!group.name || !Array.isArray(group.controls)) continue

    const controls: ControlType[] = []

    for (const suggestion of group.controls.slice(0, MAX_CONTROLS_PER_GROUP)) {
      if (!suggestion.name) continue

      if (suggestion.type === 'boolean-group') {
        const control = buildBooleanGroup(suggestion, byAddress)
        if (control) controls.push(control)
        continue
      }

      if (suggestion.type === 'boolean-enum') {
        const control = buildBooleanEnum(suggestion, byAddress)
        if (control) controls.push(control)
        continue
      }

      // Everything else (boolean/slider/step-enum) is keyed off exactly one real parameter, and
      // its kind decides the actual control type regardless of what the model picked - a
      // 'Bool' parameter can only ever honestly become a 'boolean' control, etc.
      const param = suggestion.address ? byAddress.get(suggestion.address) : undefined

      if (!param) continue

      const icon = validIcon(suggestion.icon)

      if (param.kind === 'Bool') {
        controls.push({
          id: randomUUID(),
          name: suggestion.name,
          type: 'boolean',
          icon,
          inputAddress: param.address
        })
        continue
      }

      if (param.kind === 'Float') {
        controls.push({
          id: randomUUID(),
          name: suggestion.name,
          type: 'slider',
          icon,
          inputAddress: param.address
        })
        continue
      }

      // Int: honor 'step-enum' only if the model actually gave a usable options list, otherwise
      // fall back to a plain slider - same fallback used when it explicitly picked 'slider'.
      if (
        suggestion.type === 'step-enum' &&
        Array.isArray(suggestion.options) &&
        suggestion.options.length >= 2 &&
        suggestion.options.every((o) => typeof o.name === 'string' && typeof o.value === 'number')
      ) {
        controls.push({
          id: randomUUID(),
          name: suggestion.name,
          type: 'step-enum',
          icon,
          inputAddress: param.address,
          options: suggestion.options.map((o) => ({
            name: o.name,
            value: o.value as number,
            icon: validIcon(o.icon)
          }))
        })
      } else {
        controls.push({
          id: randomUUID(),
          name: suggestion.name,
          type: 'slider',
          icon,
          inputAddress: param.address
        })
      }
    }

    if (controls.length === 0) continue

    groups.push({ id: randomUUID(), name: group.name, controls })
  }

  return groups
}
