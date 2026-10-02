import { useAvatarDetails } from '@renderer/composables/useAvatarDetails'
import { useControls } from '@renderer/composables/useControls'
import { useIntiface } from '@renderer/composables/useIntiface'
import { useOpenShock } from '@renderer/composables/useOpenShock'
import { usePresets } from '@renderer/composables/usePresets'
import { isNoisyAddress } from '@renderer/utils/addressFilter'
import { getUUID } from '@renderer/utils/uuid'
import type {
  BooleanControl,
  BooleanEnumControl,
  BooleanGroupControl,
  ControlType,
  EnumControl,
  IntifacePatternControl,
  IntifaceToyControl,
  OpenShockControl,
  PresetControl,
  SliderControl,
  StepEnumControl
} from '@toolkitosc/shared-ui'
import _ from 'lodash'
import type { ComputedRef, Ref, WritableComputedRef } from 'vue'
import { computed, ref, watch } from 'vue'

// State and rules for the control editor page (pages/ControlEditorPage.vue): the draft control,
// per-type defaults, the parameter list for the address pickers, and validation. Kept out of the
// page so its template is only about layout.
//
// Every type's fields at once, all optional: a draft changes type as the user clicks through the
// type cards, and the page reads whichever fields the current type uses. `type` is dropped from
// each before merging (the literals would intersect to `never`) and added back as the union;
// likewise `inputs`, whose shape differs between Toggle Group and Toggle Logic.
type Fields<T> = Omit<T, 'type' | 'inputs'>
type AnyDraft = Partial<
  Fields<BooleanControl> &
    Fields<BooleanGroupControl> &
    Fields<BooleanEnumControl> &
    Fields<EnumControl> &
    Fields<SliderControl> &
    Fields<StepEnumControl> &
    Fields<OpenShockControl> &
    Fields<IntifaceToyControl> &
    Fields<IntifacePatternControl> &
    Fields<PresetControl>
> & {
  type?: ControlType['type']
  inputs?: BooleanGroupControl['inputs'] | BooleanEnumControl['inputs']
}

const typeDefaults: Record<ControlType['type'], (m: AnyDraft) => void> = {
  boolean: (m) => (m.inputAddress = ''),
  slider: (m) => (m.inputAddress = ''),
  'step-enum': (m) => {
    m.inputAddress = ''
    m.options = [{ name: '', value: 0, icon: '' }]
  },
  enum: (m) => {
    m.inputAddress = ''
    m.options = [{ name: '', value: 0, icon: '' }]
  },
  'boolean-group': (m) => (m.inputs = [{ inputAddress: '' }]),
  'boolean-enum': (m) => (m.inputs = [{ id: getUUID(), name: '', icon: '', inputAddress: { true: [''], false: [''] } }]),
  'open-shock-shocker': (m) => {
    m.mode = 'Shock'
    m.shockers = []
    m.intensity = { min: 0, max: 100 }
    m.duration = { min: 300, max: 1000 }
    m.cooldown = 1000
    m.animationDuration = 3000
  },
  'intiface-toy': (m) => (m.actuators = []),
  'intiface-pattern': (m) => {
    m.actuators = []
    m.allowedPatterns = []
  },
  preset: (m) => (m.presetId = '')
}

// Which parameter type each address-based control type can drive.
const ADDRESS_TYPE: Partial<Record<ControlType['type'], 'Bool' | 'Int' | 'Float'>> = {
  boolean: 'Bool',
  'boolean-group': 'Bool',
  'boolean-enum': 'Bool',
  enum: 'Int',
  'step-enum': 'Int',
  slider: 'Float'
}

// A plain shape rather than Nuxt UI's recursive SelectMenuItem, which is too deep for TS to unwrap
// through the returned refs.
export interface ShockerOption {
  label: string
  value: string
}

export type EditorErrors = Partial<Record<'name' | 'address' | 'options' | 'inputs' | 'shockers' | 'range' | 'actuators' | 'patterns' | 'preset', string>>

export interface ControlEditor {
  model: Ref<AnyDraft>
  type: WritableComputedRef<ControlType['type'] | undefined>
  targetGroupId: Ref<string>
  isNew: boolean
  dirty: ComputedRef<boolean>
  errors: Ref<EditorErrors>
  addresses: ComputedRef<{ label: string; value: string | undefined }[]>
  showAllAddresses: Ref<boolean>
  presetId: WritableComputedRef<string>
  openShockAvailable: ComputedRef<boolean>
  intiface: { available: ComputedRef<boolean>; devices: ReturnType<typeof useIntiface>['devices'] }
  hasPresets: ComputedRef<boolean>
  shockerList: Ref<ShockerOption[]>
  previewUnavailable: ComputedRef<boolean>
  validate: () => boolean
  save: () => boolean
}

export function useControlEditor(groupId: string, controlId: string | null): ControlEditor {
  const { getControl, addControl, updateControl, deleteControl } = useControls()
  const { avatarDetails } = useAvatarDetails()
  const { getShockers, isAvailable: openShockAvailable } = useOpenShock()
  const { devices: intifaceDevices, isAvailable: intifaceAvailable } = useIntiface()
  const { presets } = usePresets()

  // `getControl` returns the computed copy with derived locked/unavailable flags - strip them so
  // they're never saved back.
  const source = controlId ? getControl(groupId, controlId) : undefined
  const initial = (): AnyDraft => {
    if (!source) return { type: 'boolean', name: '', inputAddress: '' }

    return _.omit(_.cloneDeep(source), ['locked', 'unavailable']) as AnyDraft
  }

  // Cast rather than ref<AnyDraft>(): unwrapping this wide type sends TS past its depth limit.
  const model = ref(initial()) as Ref<AnyDraft>
  const snapshot = JSON.stringify(model.value)
  const targetGroupId = ref(groupId)
  const errors = ref<EditorErrors>({})
  const showAllAddresses = ref(false)

  const dirty = computed(() => JSON.stringify(model.value) !== snapshot || targetGroupId.value !== groupId)

  const type = computed({
    get: () => model.value.type,
    set: (value) => {
      if (!value || value === model.value.type) return

      // Keep what carries over between types (name, icon, tile width, pin), reset the rest.
      const { name, icon, size, pinned, id } = model.value

      model.value = { id, name, icon, size, pinned, type: value } as AnyDraft
      typeDefaults[value](model.value)
      errors.value = {}
    }
  })

  const addresses = computed(() => {
    const wanted = model.value.type ? ADDRESS_TYPE[model.value.type] : undefined

    if (!avatarDetails.value || !wanted) return []

    return avatarDetails.value.parameters
      .filter((param) => param.input?.type === wanted && (showAllAddresses.value || !isNoisyAddress(param.name)))
      .map((param) => ({ label: param.name, value: param.input?.address }))
      .sort((a, b) => a.label.localeCompare(b.label))
  })

  // Picking a preset fills in an empty name/icon from it.
  const presetId = computed({
    get: () => model.value.presetId ?? '',
    set: (value: string) => {
      const preset = presets.value.find((p) => p.id === value)

      model.value.presetId = value
      if (preset && !model.value.name) model.value.name = preset.name
      if (preset && !model.value.icon) model.value.icon = preset.icon
    }
  })

  const shockerList = ref<ShockerOption[]>([])

  watch(
    openShockAvailable,
    (available) => {
      if (!available) {
        shockerList.value = []

        return
      }

      getShockers()
        .then((devices) => (shockerList.value = devices.flatMap((device) => device.shockers).map((shocker) => ({ label: shocker.name, value: shocker.id }))))
        .catch(() => (shockerList.value = []))
    },
    { immediate: true }
  )

  const intifaceUnavailable = computed(
    () => !intifaceAvailable.value || (model.value.actuators ?? []).every((actuator) => !intifaceDevices.value.has(actuator.deviceIndex))
  )

  const previewUnavailable = computed(
    () =>
      (model.value.type === 'open-shock-shocker' && !openShockAvailable.value) ||
      ((model.value.type === 'intiface-toy' || model.value.type === 'intiface-pattern') && intifaceUnavailable.value)
  )

  const validate = (): boolean => {
    const m = model.value
    const next: EditorErrors = {}

    if (!m.name?.trim()) next.name = 'Give the control a name.'

    switch (m.type) {
      case 'boolean':
      case 'slider':
        if (!m.inputAddress) next.address = 'Pick the parameter this control changes.'
        break
      case 'enum':
      case 'step-enum':
        if (!m.inputAddress) next.address = 'Pick the parameter this control changes.'
        if (!m.options?.length || m.options.some((option) => !option.name.trim())) next.options = 'Every option needs a name.'
        break
      case 'boolean-group':
        if (!(m.inputs as BooleanGroupControl['inputs'] | undefined)?.some((input) => input.inputAddress)) next.inputs = 'Add at least one parameter.'
        break
      case 'boolean-enum':
        if (!m.inputs?.length || (m.inputs as BooleanEnumControl['inputs']).some((input) => !input.name.trim())) next.inputs = 'Every option needs a name.'
        break
      case 'open-shock-shocker':
        if (!m.shockers?.length) next.shockers = 'Pick at least one shocker.'
        if (m.intensity && m.intensity.min > m.intensity.max) next.range = 'Minimum intensity is higher than the maximum.'
        else if (m.duration && m.duration.min > m.duration.max) next.range = 'Minimum duration is longer than the maximum.'
        break
      case 'intiface-toy':
        if (!m.actuators?.length) next.actuators = 'Pick at least one toy.'
        break
      case 'intiface-pattern':
        if (!m.actuators?.length) next.actuators = 'Pick at least one toy.'
        if (!m.allowedPatterns?.length) next.patterns = 'Pick at least one pattern.'
        break
      case 'preset':
        if (!m.presetId) next.preset = 'Pick a preset.'
        break
    }

    errors.value = next

    return Object.keys(next).length === 0
  }

  // Adds or updates the control; a changed group moves it (same id) to the end of the new group.
  const save = (): boolean => {
    if (!validate()) return false

    const control = { ...model.value, name: model.value.name!.trim() } as ControlType

    if (!control.id) {
      control.id = getUUID()
      addControl(targetGroupId.value, control)
    } else if (targetGroupId.value !== groupId) {
      deleteControl(groupId, control.id)
      addControl(targetGroupId.value, control)
    } else {
      updateControl(groupId, control)
    }

    return true
  }

  return {
    model,
    type,
    targetGroupId,
    isNew: !source,
    dirty,
    errors,
    addresses,
    showAllAddresses,
    presetId,
    openShockAvailable,
    intiface: { available: intifaceAvailable, devices: intifaceDevices },
    hasPresets: computed(() => presets.value.length > 0),
    shockerList,
    previewUnavailable,
    validate,
    save
  }
}
