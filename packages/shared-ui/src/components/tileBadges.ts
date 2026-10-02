import type { ComputedRef, InjectionKey } from 'vue'

// Small status badges Control.vue adds to whichever tile it renders (last user, "locked for
// viewers"...). Provided rather than passed as props so every control type shows them in its
// title row without each wrapper having to forward them - ControlBase injects and renders them.
export interface TileBadge {
  key: string
  label: string
  icon?: string
  avatar?: string
  tone?: 'warning' | 'muted'
}

export const TILE_BADGES: InjectionKey<ComputedRef<TileBadge[]>> = Symbol('tile-badges')
