import type { ControlType } from '../types/controls'

// How many grid cells a control's tile takes in a `.tile-grid` (see styles/aurora.css):
//   's' - one cell, 'm' - two cells wide, 'l' - two cells wide and two tall.
export type TileSpan = 's' | 'm' | 'l'

// Beyond this many options, a picker's buttons no longer fit two rows of four in a two-cell tile,
// so it gets a 2×2 tile that scrolls its options instead.
const MAX_OPTIONS_IN_WIDE_TILE = 8

function optionCount(control: ControlType): number | null {
  switch (control.type) {
    case 'enum':
      return control.options.length
    case 'boolean-enum':
      return control.inputs.length
    // +1 for the implicit "Off" pattern every pattern control shows first.
    case 'intiface-pattern':
      return control.allowedPatterns.length + 1
    default:
      return null
  }
}

// Option pickers size themselves from their option count (their buttons need the room);
// everything else is one cell unless the user asked for a wide tile.
export function controlTileSpan(control: ControlType): TileSpan {
  const options = optionCount(control)

  if (options !== null) return options > MAX_OPTIONS_IN_WIDE_TILE ? 'l' : 'm'

  return control.size === 'wide' ? 'm' : 's'
}
