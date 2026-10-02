import type { ControlType } from '@toolkitosc/shared-ui'

type Getter = <T>(address: string, fallback: T) => T

// Whether a switch-like control is currently on, from its live OSC values - `null` for types that
// have a value rather than an on/off state. Mirrors how ControlBoolean/ControlBooleanGroup read
// their own state.
export function isControlOn(control: ControlType, get: Getter): boolean | null {
  if (control.type === 'boolean') {
    const value = get<boolean>(control.inputAddress, false)

    return control.reverse ? !value : value
  }

  if (control.type === 'boolean-group') {
    return control.inputs.length > 0 && control.inputs.every((input) => get<boolean>(input.inputAddress, false))
  }

  return null
}

// Types whose tile shows a value (lit in the secondary color) rather than an on/off state.
export const VALUE_CONTROL_TYPES: ReadonlySet<ControlType['type']> = new Set([
  'slider',
  'enum',
  'step-enum',
  'boolean-enum',
  'intiface-toy',
  'intiface-pattern'
])
