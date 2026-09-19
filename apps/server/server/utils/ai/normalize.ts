import { randomUUID } from 'node:crypto'

import type { ControlGroup, ControlType } from '@vrc-osc-toolkit/shared-ui'

import type { AiParameterInput, AiSuggestionResult } from './types'

const MAX_GROUPS = 40
const MAX_CONTROLS_PER_GROUP = 60

// Turns a provider's raw suggestion into real ControlGroup[] - the safety net against a model
// that (despite the schema) hallucinates an address, gets a type/kind mismatch, or gives a
// 'step-enum' fewer than 2 options. Anything that doesn't check out is dropped rather than
// thrown on, since one bad control shouldn't sink an otherwise-good response; ids are always
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
      const param = byAddress.get(suggestion.address)

      if (!param || !suggestion.name) continue

      if (param.kind === 'Bool') {
        controls.push({
          id: randomUUID(),
          name: suggestion.name,
          type: 'boolean',
          inputAddress: param.address
        })
        continue
      }

      if (param.kind === 'Float') {
        controls.push({
          id: randomUUID(),
          name: suggestion.name,
          type: 'slider',
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
          inputAddress: param.address,
          options: suggestion.options
        })
      } else {
        controls.push({
          id: randomUUID(),
          name: suggestion.name,
          type: 'slider',
          inputAddress: param.address
        })
      }
    }

    if (controls.length === 0) continue

    groups.push({ id: randomUUID(), name: group.name, controls })
  }

  return groups
}
