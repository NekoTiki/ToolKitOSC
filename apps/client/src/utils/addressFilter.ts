// Hardcoded denylist of known-noisy avatar parameter namespaces (PhysBones-adjacent/VRCFury/
// OSCGoodies-style prefixes), shared between the control editor's address picker (ControlModal.vue)
// and the preset creator (usePresets.ts) - previously duplicated as ControlModal.vue's own private
// `isFilteredAddress`, now the one definition both read from.
const NOISY_ADDRESS_PREFIXES = [
  'pcs/',
  'WH Lollipop/',
  'OGB/',
  'Go/',
  'FT/',
  '_ShockButton',
  'Cam/',
  'Dor/',
]
const NOISY_ADDRESS_PATTERN = /VF\d+_/

// VRChat auto-generates these five per PhysBone component, appended to whichever parameter name
// the avatar creator picked for it (`{parameter}_IsGrabbed`, etc.) - always noisy, regardless of
// the base parameter name, so matched as a suffix rather than added to the prefix/pattern lists
// above (which match the start of the name instead).
const NOISY_ADDRESS_SUFFIXES = ['_IsGrabbed', '_IsPosed', '_Angle', '_Stretch', '_Squish']

export const isNoisyAddress = (name: string): boolean => {
  if (NOISY_ADDRESS_PREFIXES.some((prefix) => name.startsWith(prefix))) return true
  if (NOISY_ADDRESS_SUFFIXES.some((suffix) => name.endsWith(suffix))) return true

  return NOISY_ADDRESS_PATTERN.test(name)
}
