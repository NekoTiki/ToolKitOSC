import { useAvatarDetails } from '@renderer/composables/useAvatarDetails'
import { useOscMessages } from '@renderer/composables/useOscMessages'
import { api } from '@renderer/lib/tauri-bridge'
import { isNoisyAddress } from '@renderer/utils/addressFilter'
import type { OSCArg, Preset, PresetParameter } from '@vrc-osc-toolkit/shared-ui'
import type { ComputedRef } from 'vue'
import { computed, ref } from 'vue'

// One row per input-addressable parameter VRChat declared for the current avatar, joined with
// whatever we've actually captured for it so far - the single source both the coverage banner and
// PresetModal's parameter table read from.
export type FilterableParameter = {
  address: string
  name: string
  kind: PresetParameter['kind']
  captured: boolean
  value?: OSCArg
  excluded: boolean
}

const presets = ref<Preset[]>([])
const excludedAddresses = ref<string[]>([])
// Guards against a slow load_presets response landing after the user has already switched to a
// different avatar (or switched again before the first one resolved) - only the response for the
// avatar we're still on is allowed to overwrite `presets`/`excludedAddresses`.
const loadedAvatarId = ref<string | null>(null)

export function usePresets(): {
  presets: typeof presets
  filterableParameters: ComputedRef<FilterableParameter[]>
  includedParameters: ComputedRef<FilterableParameter[]>
  coverage: ComputedRef<{ captured: number; total: number; ratio: number }>
  isExcluded: (address: string) => boolean
  setExcluded: (address: string, excluded: boolean) => void
  captureParameters: () => PresetParameter[]
  getPreset: (id: string) => Preset | undefined
  // Returns the new preset's id, or null when no avatar is loaded.
  addPreset: (name: string, icon: string | undefined, parameters: PresetParameter[]) => string | null
  updatePreset: (
    id: string,
    patch: Partial<Pick<Preset, 'name' | 'icon' | 'parameters'>>
  ) => void
  deletePreset: (id: string) => void
  applyPresetParameters: (parameters: PresetParameter[]) => void
  applyPreset: (id: string) => void
} {
  const { avatarDetails } = useAvatarDetails((details) => loadForAvatar(details.id))
  const { args, update } = useOscMessages()

  const avatarId = computed(() => avatarDetails.value?.id)

  const loadForAvatar = (id: string): void => {
    loadedAvatarId.value = id

    void api.loadPresets(id).then((store) => {
      if (loadedAvatarId.value !== id) return

      presets.value = store.presets
      excludedAddresses.value = store.excludedAddresses
    })
  }

  const persist = (): void => {
    if (!avatarId.value) return

    void api.savePresets(avatarId.value, {
      excludedAddresses: excludedAddresses.value,
      presets: presets.value
    })
  }

  const filterableParameters = computed<FilterableParameter[]>(() => {
    if (!avatarDetails.value) return []

    return avatarDetails.value.parameters
      .filter((param) => !!param.input)
      .map((param) => {
        const address = param.input!.address

        return {
          address,
          name: param.name,
          kind: param.input!.type,
          captured: address in args.value,
          value: args.value[address]?.[0],
          excluded: excludedAddresses.value.includes(address)
        }
      })
      .sort((a, b) => a.name.localeCompare(b.name))
  })

  // What "enough parameters received" is judged against, and what the parameter picker offers by
  // default: everything not hidden by the hardcoded noisy-address denylist or this avatar's own
  // custom exclude list. A caller that needs the full unfiltered list (e.g. a "show all" toggle,
  // or to manage the exclude list itself) reads `filterableParameters` directly instead.
  const includedParameters = computed(() =>
    filterableParameters.value.filter((param) => !param.excluded && !isNoisyAddress(param.name))
  )

  const coverage = computed(() => {
    const total = includedParameters.value.length
    const captured = includedParameters.value.filter((param) => param.captured).length

    return { total, captured, ratio: total === 0 ? 0 : captured / total }
  })

  const isExcluded = (address: string): boolean => excludedAddresses.value.includes(address)

  const setExcluded = (address: string, excluded: boolean): void => {
    if (excluded) {
      if (!excludedAddresses.value.includes(address)) excludedAddresses.value.push(address)
    } else {
      excludedAddresses.value = excludedAddresses.value.filter((a) => a !== address)
    }

    persist()
  }

  const captureParameters = (): PresetParameter[] => {
    return includedParameters.value
      .filter((param) => param.captured)
      .map((param) => ({
        address: param.address,
        name: param.name,
        kind: param.kind,
        value: param.value as OSCArg
      }))
  }

  const getPreset = (id: string): Preset | undefined => presets.value.find((p) => p.id === id)

  const addPreset = (name: string, icon: string | undefined, parameters: PresetParameter[]): string | null => {
    if (!avatarId.value) return null

    const now = new Date().toISOString()
    const id = self.crypto.randomUUID()

    presets.value.push({
      id,
      avatarId: avatarId.value,
      avatarName: avatarDetails.value?.name ?? '',
      name,
      icon,
      createdAt: now,
      updatedAt: now,
      parameters
    })

    persist()

    return id
  }

  const updatePreset = (
    id: string,
    patch: Partial<Pick<Preset, 'name' | 'icon' | 'parameters'>>
  ): void => {
    const preset = getPreset(id)

    if (!preset) return

    Object.assign(preset, patch, { updatedAt: new Date().toISOString() })

    persist()
  }

  const deletePreset = (id: string): void => {
    presets.value = presets.value.filter((p) => p.id !== id)

    persist()
  }

  const applyPresetParameters = (parameters: PresetParameter[]): void => {
    parameters.forEach((param) => update({ address: param.address, args: [param.value] }))
  }

  const applyPreset = (id: string): void => {
    const preset = getPreset(id)

    if (preset) applyPresetParameters(preset.parameters)
  }

  return {
    presets,
    filterableParameters,
    includedParameters,
    coverage,
    isExcluded,
    setExcluded,
    captureParameters,
    getPreset,
    addPreset,
    updatePreset,
    deletePreset,
    applyPresetParameters,
    applyPreset
  }
}
