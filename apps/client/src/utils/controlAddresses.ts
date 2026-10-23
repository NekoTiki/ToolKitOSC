import type { ControlType } from '@toolkitosc/shared-ui'

// The avatar parameters a control drives, by type.
const inputAddresses = (control: ControlType): string[] => {
  switch (control.type) {
    case 'boolean':
    case 'enum':
    case 'step-enum':
    case 'slider':
      return [control.inputAddress]
    case 'boolean-group':
      return control.inputs.map((input) => input.inputAddress)
    case 'boolean-enum':
      return control.inputs.flatMap((input) => [...input.inputAddress.true, ...input.inputAddress.false])
    default:
      return []
  }
}

// Which control drives each avatar parameter, by full OSC address. The last one wins when two do.
export const controlsByAddress = (controls: ControlType[]): Map<string, ControlType> => {
  const byAddress = new Map<string, ControlType>()
  for (const control of controls) inputAddresses(control).forEach((address) => byAddress.set(address, control))
  return byAddress
}
