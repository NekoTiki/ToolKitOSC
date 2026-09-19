import type { AiParameterInput } from './types'

// The stem a parameter's name would share with siblings meant to be read as one category - the
// name up to (not including) its last '/' or '_' separator. "VF8_Handjob/Handjob Left" and
// "VF8_Handjob/Handjob Right" -> "VF8_Handjob". A name with no separator has no stem (null).
function stemOf(name: string): string | null {
  const match = /^(.*)[/_][^/_]+$/.exec(name)

  return match?.[1] ?? null
}

// Deterministic candidates for Balanced/Heavy's boolean-group/boolean-enum hints (see
// profiles.ts's useClusterHints, prompt.ts), handed to the model as suggestions it still has to
// judge - never auto-applied. Two signals, both grounded in the avatar's own declared parameter
// names rather than a guess about what a naming pattern probably means:
//
// 1. An Int-kind parameter sharing a name stem with a cluster of Bool-kind parameters - a common
//    VRCFury pattern where an Int-driven selector is exposed redundantly as parallel Bool toggles
//    for OSC-menu convenience.
// 2. A cluster of Bool-kind parameters sharing a name stem with no Int sibling at all - the more
//    common real-world case (e.g. "Top_1"/"Top_2"/"Top_3") and the one most likely to actually
//    want boolean-group/boolean-enum, but with no Int selector to confirm exclusivity from, so the
//    hint hands the model both possibilities (mutually-exclusive vs. always-together) to judge
//    from the names alone instead of only ever mentioning the narrower Int+Bool case.
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

  for (const [stem, group] of byStem) {
    const intSibling = group.find((p) => p.kind === 'Int')
    const boolSiblings = group.filter((p) => p.kind === 'Bool')

    if (boolSiblings.length < 2) continue

    const boolList = boolSiblings.map((p) => `"${p.name}" (${p.address})`).join(', ')

    if (intSibling) {
      hints.push(
        `"${intSibling.name}" (Int, ${intSibling.address}) looks like a selector with these Bool parameters as its options: ${boolList} - likely a mutually-exclusive set (candidate for boolean-enum), or the Int itself is the better single step-enum control. Verify from the names before using either.`
      )
    } else {
      hints.push(
        `These Bool parameters share a common name stem "${stem}" with no Int selector: ${boolList} - candidate for boolean-enum if they're mutually-exclusive alternatives in the same slot (e.g. different styles/variants), or boolean-group if they're always toggled together as one unit. Judge which, if either, from the names - don't force a relationship that isn't really there.`
      )
    }
  }

  return hints
}
