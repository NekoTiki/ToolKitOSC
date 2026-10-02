import { randomUUID } from 'node:crypto'

import type { ControlGroup, ControlType } from '@toolkitosc/shared-ui'

import { ICON_CHOICES } from './prompt'
import { aiAddressSchema, aiControlNameSchema, stepEnumOptionSchema } from './schemas'
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
// (trueAddresses/falseAddresses), which both only make sense over boolean parameters. Each address
// goes through the same aiAddressSchema every other address in this file does, trimmed before the
// lookup - a model that echoed one back with incidental whitespace still matches instead of
// silently failing to find it.
function validBoolAddresses(
  addresses: unknown,
  byAddress: Map<string, AiParameterInput>
): string[] {
  if (!Array.isArray(addresses)) return []

  const valid: string[] = []

  for (const address of addresses) {
    const parsed = aiAddressSchema.safeParse(address)
    if (parsed.success && byAddress.get(parsed.data)?.kind === 'Bool') valid.push(parsed.data)
  }

  return valid
}

function buildBooleanGroup(
  name: string,
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
      name,
      type: 'boolean',
      icon: validIcon(suggestion.icon),
      inputAddress: singleAddress
    }
  }

  if (addresses.length < 2) return null

  return {
    id: randomUUID(),
    name,
    type: 'boolean-group',
    icon: validIcon(suggestion.icon),
    inputs: addresses.map((inputAddress) => ({ inputAddress }))
  }
}

function buildBooleanEnum(
  name: string,
  suggestion: AiControlSuggestion,
  byAddress: Map<string, AiParameterInput>
): ControlType | null {
  if (!Array.isArray(suggestion.options)) return null

  const inputs = suggestion.options
    .map((option) => {
      const parsedOptionName = aiControlNameSchema.safeParse(option.name)
      const trueAddresses = validBoolAddresses(option.trueAddresses, byAddress)

      // No verified name or "on" address at all means this option is pure hallucination - drop
      // just this option rather than the whole control, same "one bad item doesn't sink the rest"
      // approach as everywhere else here.
      if (!parsedOptionName.success || trueAddresses.length === 0) return null

      return {
        id: randomUUID(),
        name: parsedOptionName.data,
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
    name,
    type: 'boolean-enum',
    icon: validIcon(suggestion.icon),
    inputs
  }
}

// Validates a 'step-enum' suggestion's raw `options` with zod, per option - a bad/hallucinated
// option (missing name, non-integer or out-of-range value) is dropped on its own rather than
// failing the whole control, the same "one bad item doesn't sink the rest" approach as
// buildBooleanEnum above. Two duplicate-valued options would also both light up as "current" at
// once in the picker (see shared-ui's ControlEnum.vue), so a value seen already is dropped too,
// keeping the first (usually the model's more specific/first-mentioned) name for it.
function buildStepEnumOptions(options: unknown): { name: string; value: number; icon?: string }[] | null {
  if (!Array.isArray(options)) return null

  const seenValues = new Set<number>()
  const valid: { name: string; value: number; icon?: string }[] = []

  for (const option of options) {
    const parsed = stepEnumOptionSchema.safeParse(option)
    if (!parsed.success || seenValues.has(parsed.data.value)) continue

    seenValues.add(parsed.data.value)
    valid.push({ name: parsed.data.name, value: parsed.data.value, icon: validIcon(parsed.data.icon) })
  }

  // A "step" needs at least two real values to step between - can't honestly call this an enum
  // control with zero or one surviving option, so the caller falls back to a plain slider instead.
  return valid.length >= 2 ? valid : null
}

// Turns a provider's raw suggestion into real ControlGroup[] - the safety net against a model
// that (despite the schema) hallucinates an address, gets a type/kind mismatch, or gives a
// 'step-enum'/'boolean-enum' too few options. Every name and address that ends up on a real
// ControlType has gone through the shared zod leaf schemas above (aiControlNameSchema/
// aiAddressSchema/stepEnumOptionSchema) rather than an ad-hoc truthy/typeof check - a group or
// control whose own name doesn't check out is dropped outright, while a bad item nested inside an
// otherwise-fine control (one boolean-enum option, one step-enum value) is dropped on its own so it
// doesn't sink the rest. Ids are always generated here, never trusted from the model.
export function normalizeSuggestion(
  result: AiSuggestionResult,
  parameters: AiParameterInput[]
): ControlGroup[] {
  const byAddress = new Map(parameters.map((p) => [p.address, p]))

  const groups: ControlGroup[] = []

  for (const group of result.groups.slice(0, MAX_GROUPS)) {
    const groupName = aiControlNameSchema.safeParse(group.name)
    if (!groupName.success || !Array.isArray(group.controls)) continue

    const controls: ControlType[] = []

    for (const suggestion of group.controls.slice(0, MAX_CONTROLS_PER_GROUP)) {
      const controlName = aiControlNameSchema.safeParse(suggestion.name)
      if (!controlName.success) continue

      const name = controlName.data

      if (suggestion.type === 'boolean-group') {
        const control = buildBooleanGroup(name, suggestion, byAddress)
        if (control) controls.push(control)
        continue
      }

      if (suggestion.type === 'boolean-enum') {
        const control = buildBooleanEnum(name, suggestion, byAddress)
        if (control) controls.push(control)
        continue
      }

      // Everything else (boolean/slider/step-enum) is keyed off exactly one real parameter, and
      // its kind decides the actual control type regardless of what the model picked - a
      // 'Bool' parameter can only ever honestly become a 'boolean' control, etc.
      const addressResult = aiAddressSchema.safeParse(suggestion.address)
      const param = addressResult.success ? byAddress.get(addressResult.data) : undefined

      if (!param) continue

      const icon = validIcon(suggestion.icon)

      if (param.kind === 'Bool') {
        controls.push({
          id: randomUUID(),
          name,
          type: 'boolean',
          icon,
          inputAddress: param.address
        })
        continue
      }

      if (param.kind === 'Float') {
        controls.push({
          id: randomUUID(),
          name,
          type: 'slider',
          icon,
          inputAddress: param.address
        })
        continue
      }

      // Int: honor 'step-enum' only if the model actually gave a usable options list (validated
      // and repaired by buildStepEnumOptions above), otherwise fall back to a plain slider - same
      // fallback used when it explicitly picked 'slider'.
      const stepEnumOptions = suggestion.type === 'step-enum' ? buildStepEnumOptions(suggestion.options) : null

      if (stepEnumOptions) {
        controls.push({
          id: randomUUID(),
          name,
          type: 'step-enum',
          icon,
          inputAddress: param.address,
          options: stepEnumOptions
        })
      } else {
        controls.push({
          id: randomUUID(),
          name,
          type: 'slider',
          icon,
          inputAddress: param.address
        })
      }
    }

    if (controls.length === 0) continue

    groups.push({ id: randomUUID(), name: groupName.data, controls })
  }

  return groups
}
