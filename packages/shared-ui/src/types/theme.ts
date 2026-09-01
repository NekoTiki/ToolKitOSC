// Pure theme constants/types, no DOM usage — lives here (not in composables/useTheme.ts) so
// that type-only consumers (e.g. protocol.ts, and apps/server's Nitro-side type re-export,
// which runs under a tsconfig with no DOM lib) can depend on these without dragging in
// useTheme.ts's document-touching function bodies.
export const SHADES = ['50', '100', '200', '300', '400', '500', '600', '700', '800', '900'] as const

export const COLOR_FAMILIES = [
  'red',
  'orange',
  'amber',
  'yellow',
  'lime',
  'green',
  'emerald',
  'teal',
  'cyan',
  'sky',
  'blue',
  'indigo',
  'violet',
  'purple',
  'fuchsia',
  'pink',
  'rose'
] as const
export const NEUTRAL_COLOR_FAMILIES = ['slate', 'gray', 'zinc', 'neutral'] as const
export type ColorFamily = (typeof COLOR_FAMILIES)[number] | (typeof NEUTRAL_COLOR_FAMILIES)[number]
export type ColorFamilyWithoutNeutral = (typeof COLOR_FAMILIES)[number]
