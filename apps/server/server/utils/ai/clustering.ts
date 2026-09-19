import type { AiParameterInput } from './types'

// The stem a parameter's name would share with siblings meant to be read as one category - the
// name up to (not including) its last '/' or '_' separator. "VF8_Handjob/Handjob Left" and
// "VF8_Handjob/Handjob Right" -> "VF8_Handjob". A name with no separator has no stem (null).
function stemOf(name: string): string | null {
  const match = /^(.*)[/_][^/_]+$/.exec(name)

  return match?.[1] ?? null
}

// Deterministic candidates for Heavy mode's boolean-group/boolean-enum hints, handed to the model
// as suggestions it still has to judge - never auto-applied (see prompt.ts). Only one signal is
// implemented here (see the project's prior conceptual writeup for others considered and skipped):
// an Int-kind parameter sharing a name stem with a cluster of Bool-kind parameters. This is the
// strongest signal available from names alone because it's grounded in the avatar's own declared
// structure (a common VRCFury pattern: an Int-driven selector exposed redundantly as parallel Bool
// toggles for OSC-menu convenience) rather than a guess about what a naming pattern probably means.
export function detectClusterHints(parameters: AiParameterInput[]): string[] {
  const byStem = new Map<string, AiParameterInput[]>()

  for (const param of parameters) {
    const stem = stemOf(param.name)

    if (!stem) continue

    const group = byStem.get(stem) ?? []
    group.push(param)
    byStem.set(stem, group)
  }

  const hints: string[] = []

  for (const group of byStem.values()) {
    const intSibling = group.find((p) => p.kind === 'Int')
    const boolSiblings = group.filter((p) => p.kind === 'Bool')

    if (!intSibling || boolSiblings.length < 2) continue

    hints.push(
      `"${intSibling.name}" (Int, ${intSibling.address}) looks like a selector with these Bool parameters as its options: ${boolSiblings.map((p) => `"${p.name}" (${p.address})`).join(', ')} - likely a mutually-exclusive set (candidate for boolean-enum), or the Int itself is the better single step-enum control. Verify from the names before using either.`
    )
  }

  return hints
}
