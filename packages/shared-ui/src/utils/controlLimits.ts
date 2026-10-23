import type { ControlLimits, ControlType, ViewerGroupInfo } from '../types/controls'

const clamp = (value: number, min: number, max: number): number => Math.min(max, Math.max(min, value))

export const isOptionDisabled = (limits: ControlLimits | undefined, key: string | number): boolean =>
  !!limits?.disabledOptions?.includes(key)

// The range a viewer may set on a Slider or Toy, 0-1.
export const limitRange = (limits: ControlLimits | undefined): { min: number; max: number } => {
  const min = clamp(limits?.min ?? 0, 0, 1)

  return { min, max: clamp(limits?.max ?? 1, min, 1) }
}

// Drops the parts of a ControlLimits that don't limit anything (full range, nothing disabled), so
// "no limits" is always stored and sent as `undefined`.
export const normalizeLimits = (limits: ControlLimits | undefined): ControlLimits | undefined => {
  if (!limits) return undefined

  const normalized: ControlLimits = {}

  if (limits.min !== undefined && limits.min > 0) normalized.min = clamp(limits.min, 0, 1)
  if (limits.max !== undefined && limits.max < 1) normalized.max = clamp(limits.max, normalized.min ?? 0, 1)
  if (limits.disabledOptions?.length) normalized.disabledOptions = [...limits.disabledOptions]

  return Object.keys(normalized).length ? normalized : undefined
}

// The most generous of several limits: the lowest min, the highest max, and only the options every
// one of them blocks. Any `undefined` (no limits) wins outright.
export const mostGenerousLimits = (list: (ControlLimits | undefined)[]): ControlLimits | undefined => {
  if (!list.length || list.some((limits) => !normalizeLimits(limits))) return undefined

  const ranges = list.map(limitRange)
  const [first, ...rest] = list.map((limits) => limits?.disabledOptions ?? [])

  return normalizeLimits({
    min: Math.min(...ranges.map((range) => range.min)),
    max: Math.max(...ranges.map((range) => range.max)),
    disabledOptions: (first ?? []).filter((key) => rest.every((keys) => keys.includes(key)))
  })
}

// One viewer's copy of a control: the active profile's rules for everyone, opened up by the rules
// of the groups the viewer is in (`groupIds`), the most generous one winning. `groups` names those
// groups, for the share page's badge. The returned control never carries `groupRules`, so it can go
// to a viewer as-is. `unavailable` is left alone: no group can use a toy that's offline.
export function resolveControlForViewer<T extends ControlType>(control: T, groupIds: readonly string[], groups: readonly ViewerGroupInfo[] = []): T {
  const { groupRules, ...base } = control

  delete base.viaGroup

  const everyone = { locked: !!control.locked, limits: normalizeLimits(control.limits) }
  const candidates = groupIds
    .filter((id) => groupRules?.[id])
    .map((id) => {
      const rule = groupRules![id]!

      return { id, locked: rule.locked ?? everyone.locked, limits: rule.limits ? normalizeLimits(rule.limits) : everyone.limits }
    })
    .filter((candidate) => !candidate.locked)

  if (!candidates.length) return base as T

  const open = everyone.locked ? candidates : [{ id: null, ...everyone }, ...candidates]
  const limits = mostGenerousLimits(open.map((candidate) => candidate.limits))
  const changed = everyone.locked || JSON.stringify(limits) !== JSON.stringify(everyone.limits)
  // The first group whose own rule is part of what this viewer gets.
  const via = changed ? groups.find((group) => candidates.some((candidate) => candidate.id === group.id)) : undefined

  return {
    ...base,
    locked: false,
    limits,
    ...(via ? { viaGroup: { name: via.name, color: via.color } } : {})
  } as T
}
