<script setup lang="ts">
import type { SelectMenuItem } from '@nuxt/ui/components/SelectMenu.vue'
import AddressField from '@renderer/components/control-modal/AddressField.vue'
import BooleanEnumFields from '@renderer/components/control-modal/BooleanEnumFields.vue'
import BooleanGroupFields from '@renderer/components/control-modal/BooleanGroupFields.vue'
import EnumOptionsField from '@renderer/components/control-modal/EnumOptionsField.vue'
import IntifaceFields from '@renderer/components/control-modal/IntifaceFields.vue'
import IntifacePatternFields from '@renderer/components/control-modal/IntifacePatternFields.vue'
import OpenShockFields from '@renderer/components/control-modal/OpenShockFields.vue'
import PresetField from '@renderer/components/control-modal/PresetField.vue'
import IconSelectMenu from '@renderer/components/IconSelectMenu.vue'
import { useAvatarDetails } from '@renderer/composables/useAvatarDetails'
import { getUUID, useControlModal } from '@renderer/composables/useControlModal'
import { useControls } from '@renderer/composables/useControls'
import { useIntiface } from '@renderer/composables/useIntiface'
import { useOpenShock } from '@renderer/composables/useOpenShock'
import { usePresets } from '@renderer/composables/usePresets'
import { isNoisyAddress } from '@renderer/utils/addressFilter'
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
} from '@vrc-osc-toolkit/shared-ui'
import { Control, CONTROL_TYPE_LABELS } from '@vrc-osc-toolkit/shared-ui'
import { computed, ref, watch } from 'vue'

const { model, open, submit } = useControlModal()
const { avatarDetails } = useAvatarDetails()
const { handleCommand } = useControls()
const { getShockers, isAvailable: openShockAvailable } = useOpenShock()
const { devices: intifaceDevices, isAvailable: intifaceAvailable } = useIntiface()
const { presets } = usePresets()

type SelectMenuItemType = SelectMenuItem & { value?: ControlType['type']; disabled?: boolean }
type SelectMenuItemOpenShockMode = SelectMenuItem & { value: OpenShockControl['mode'] }

// A computed (not a plain ref) so the "Shocker" option flips back to selectable the moment a
// valid API key is configured in Settings, without the user needing to reopen this modal.
// Grouped (an array of arrays, per USelect's group support) by which system each type talks to,
// with a `label`-type item heading each group and a `separator` dividing them.
const types = computed<SelectMenuItemType[][]>(() => [
  [
    { type: 'label', label: 'VRChat' },
    { label: CONTROL_TYPE_LABELS.boolean, value: 'boolean' },
    { label: CONTROL_TYPE_LABELS['boolean-group'], value: 'boolean-group' },
    { label: CONTROL_TYPE_LABELS['boolean-enum'], value: 'boolean-enum' },
    { label: CONTROL_TYPE_LABELS.enum, value: 'enum' },
    { label: CONTROL_TYPE_LABELS.slider, value: 'slider' }
  ],
  [
    { type: 'separator' },
    { type: 'label', label: 'OpenShock' },
    {
      label: CONTROL_TYPE_LABELS['open-shock-shocker'],
      value: 'open-shock-shocker',
      disabled: !openShockAvailable.value
    }
  ],
  [
    { type: 'separator' },
    { type: 'label', label: 'Intiface' },
    {
      label: CONTROL_TYPE_LABELS['intiface-toy'],
      value: 'intiface-toy',
      disabled: !intifaceAvailable.value
    },
    {
      label: CONTROL_TYPE_LABELS['intiface-pattern'],
      value: 'intiface-pattern',
      disabled: !intifaceAvailable.value
    }
  ],
  [
    { type: 'separator' },
    { type: 'label', label: 'Presets' },
    { label: CONTROL_TYPE_LABELS.preset, value: 'preset', disabled: !presets.value.length }
  ]
])

const openShockMode = ref<SelectMenuItemOpenShockMode[]>([
  { label: 'Shock', value: 'Shock' },
  { label: 'Vibrate', value: 'Vibrate' }
])

// Per-type defaults applied when switching "Type" - keeps the fields each control type needs
// pre-populated so its editor (and the live preview) has valid data immediately. `model` is
// `Partial<ControlType>`, a union that only exposes fields common to every variant, so each
// branch casts to the narrow shape it actually initializes.
const typeDefaults: Record<ControlType['type'], (m: Partial<ControlType>) => void> = {
  boolean: (m) => {
    const booleanModel = m as Partial<BooleanControl>

    booleanModel.inputAddress = ''
  },
  slider: (m) => {
    const sliderModel = m as Partial<SliderControl>

    sliderModel.inputAddress = ''
  },
  'step-enum': (m) => {
    const stepEnumModel = m as Partial<StepEnumControl>

    stepEnumModel.inputAddress = ''
  },
  enum: (m) => {
    const enumModel = m as Partial<EnumControl>

    enumModel.inputAddress = ''
    enumModel.options = [{ name: '', value: 0, icon: '' }]
  },
  'boolean-group': (m) => {
    const groupModel = m as Partial<BooleanGroupControl>

    groupModel.inputs = [{ inputAddress: '' }]
  },
  'boolean-enum': (m) => {
    const booleanEnumModel = m as Partial<BooleanEnumControl>

    booleanEnumModel.inputs = [
      { id: getUUID(), name: '', icon: '', inputAddress: { true: [''], false: [''] } }
    ]
  },
  'open-shock-shocker': (m) => {
    const openShockModel = m as Partial<OpenShockControl>

    openShockModel.mode = 'Shock'
    openShockModel.shockers = []
    openShockModel.intensity = { min: 0, max: 100 }
    openShockModel.duration = { min: 300, max: 1000 }
    openShockModel.cooldown = 1000
    openShockModel.animationDuration = 3000
  },
  'intiface-toy': (m) => {
    const intifaceModel = m as Partial<IntifaceToyControl>

    intifaceModel.actuators = []
  },
  'intiface-pattern': (m) => {
    const intifacePatternModel = m as Partial<IntifacePatternControl>

    intifacePatternModel.actuators = []
    intifacePatternModel.allowedPatterns = []
  },
  preset: (m) => {
    const presetModel = m as Partial<PresetControl>

    presetModel.presetId = ''
  }
}

const presetControlId = computed<PresetControl['presetId']>({
  get: () => (model.value as Partial<PresetControl>).presetId ?? '',
  set: (val) => {
    const presetModel = model.value as Partial<PresetControl>
    const preset = presets.value.find((p) => p.id === val)

    presetModel.presetId = val

    // Defaults the control's own name/icon from the preset the first time one is picked, without
    // overwriting whatever the user already typed.
    if (preset && !model.value.name) model.value.name = preset.name
    if (preset && !model.value.icon) model.value.icon = preset.icon
  }
})

const modelType = computed({
  get: () => model.value.type,
  set: (val: ControlType['type']) => {
    model.value.type = val
    typeDefaults[val]?.(model.value)
  }
})

const showAllAddresses = ref(false)

const addresses = computed(() => {
  if (!avatarDetails.value) return []

  return avatarDetails.value.parameters
    .filter((param) => {
      if (!param.input) return false

      let typeCheck = false
      const type = (model.value.type || '') as ControlType['type']

      if (['boolean', 'boolean-group', 'boolean-enum'].includes(type)) {
        typeCheck = param.input.type === 'Bool'
      } else if (['enum', 'step-enum'].includes(type)) {
        typeCheck = param.input.type === 'Int'
      } else if (['slider'].includes(type)) {
        typeCheck = param.input.type === 'Float'
      }

      if (!typeCheck) return false

      return showAllAddresses.value || !isNoisyAddress(param.name)
    })
    .map((param) => ({
      label: param.name,
      value: param.input?.address
      // TODO: UI bugged as of version 2.1.0 of Nuxt UI
      // description: `Current Value: ${get<unknown>(param.input?.address || '', false)}`
    }))
    .sort((a, b) => a.label.localeCompare(b.label))
})

// Narrow proxies onto `model` (typed `Partial<ControlType>`) so the field subcomponents get a
// concrete, single-variant prop type instead of the full control union.
const booleanGroupInputs = computed<BooleanGroupControl['inputs']>({
  get: () => (model.value as Partial<BooleanGroupControl>).inputs ?? [],
  set: (val) => {
    const groupModel = model.value as Partial<BooleanGroupControl>

    groupModel.inputs = val
  }
})

const booleanEnumInputs = computed<BooleanEnumControl['inputs']>({
  get: () => (model.value as Partial<BooleanEnumControl>).inputs ?? [],
  set: (val) => {
    const booleanEnumModel = model.value as Partial<BooleanEnumControl>

    booleanEnumModel.inputs = val
  }
})

const enumOptions = computed<EnumControl['options']>({
  get: () => (model.value as Partial<EnumControl>).options ?? [],
  set: (val) => {
    const enumModel = model.value as Partial<EnumControl>

    enumModel.options = val
  }
})

const intifaceActuators = computed<IntifaceToyControl['actuators']>({
  get: () => (model.value as Partial<IntifaceToyControl>).actuators ?? [],
  set: (val) => {
    const intifaceModel = model.value as Partial<IntifaceToyControl>

    intifaceModel.actuators = val
  }
})

// IntifacePatternControl's `actuators` field is the exact same shape as IntifaceToyControl's - a
// separate proxy only so each field subcomponent still gets a single, concrete variant type to
// work with (matching every other proxy here), not because the underlying data differs.
const intifacePatternActuators = computed<IntifacePatternControl['actuators']>({
  get: () => (model.value as Partial<IntifacePatternControl>).actuators ?? [],
  set: (val) => {
    const intifacePatternModel = model.value as Partial<IntifacePatternControl>

    intifacePatternModel.actuators = val
  }
})

const intifaceAllowedPatterns = computed<IntifacePatternControl['allowedPatterns']>({
  get: () => (model.value as Partial<IntifacePatternControl>).allowedPatterns ?? [],
  set: (val) => {
    const intifacePatternModel = model.value as Partial<IntifacePatternControl>

    intifacePatternModel.allowedPatterns = val
  }
})

// Mirrors useControls.ts's real unavailable-gating for 'intiface-toy'/'intiface-pattern' (both
// share the exact same `actuators` shape), so the live preview shows the same state a viewer would
// actually see: unavailable while Intiface itself isn't connected, or once every toy this control
// targets has been unplugged/disconnected.
const intifaceControlUnavailable = computed(() => {
  if (!intifaceAvailable.value) return true

  const actuators =
    (model.value as Partial<IntifaceToyControl> & Partial<IntifacePatternControl>).actuators ?? []

  return actuators.every((actuator) => !intifaceDevices.value.has(actuator.deviceIndex))
})

const shockerList = ref<SelectMenuItem[]>([])

// `openShockAvailable` starts out false and only flips true once the token check (an async
// network request) resolves - so this can't be a one-shot onMounted fetch, or it fires while the
// check is still pending, sees `false`, and never retries even after the token is confirmed valid
// moments later. Watching (immediate, so a component instance that mounts after the check already
// resolved still fetches once) re-fetches every time availability flips true, including after the
// user fixes the key in Settings while this modal is still open.
watch(
  openShockAvailable,
  (available) => {
    if (!available) {
      shockerList.value = []
      return
    }

    getShockers()
      .then((shockers) => {
        shockerList.value = shockers
          .map((shocker) => shocker.shockers)
          .flat()
          .map((shocker) => ({
            label: shocker.name,
            value: shocker.id
          }))
      })
      .catch(() => {
        shockerList.value = []
      })
  },
  { immediate: true }
)
</script>

<template>
  <UModal
    v-model:open="open"
    :ui="{ content: 'max-w-4xl' }"
    :dismissible="false"
  >
    <template #content>
      <UCard :ui="{ root: 'overflow-auto', body: 'grid grid-cols-[1fr_220px] gap-4 max-h-full' }">
        <UForm class="flex h-min max-h-full grow flex-col gap-2">
          <UFormField
            label="Type"
            required
          >
            <USelect
              v-model="modelType"
              :items="types"
              class="w-full"
            />
          </UFormField>
          <UFormField
            label="Name"
            required
          >
            <UInput
              v-model="model.name"
              class="w-full"
            />
          </UFormField>
          <UFormField
            v-if="model.type !== 'open-shock-shocker'"
            label="Icon"
          >
            <IconSelectMenu
              v-model="model.icon"
              class="w-full"
              @keyup.backspace="model.icon = ''"
            />
          </UFormField>

          <template
            v-if="
              model.type === 'boolean' ||
                model.type === 'enum' ||
                model.type === 'slider' ||
                model.type === 'step-enum'
            "
          >
            <AddressField
              v-model:address="model.inputAddress"
              v-model:show-all="showAllAddresses"
              :addresses="addresses"
            />
            <UCheckbox
              v-if="model.type === 'boolean'"
              v-model="model.reverse"
              label="Reverse Mode"
              description="(On = Off, Off = On)"
            />
          </template>

          <BooleanGroupFields
            v-if="model.type === 'boolean-group'"
            v-model:inputs="booleanGroupInputs"
            v-model:show-all="showAllAddresses"
            :addresses="addresses"
          />

          <BooleanEnumFields
            v-if="model.type === 'boolean-enum'"
            v-model:inputs="booleanEnumInputs"
            v-model:show-all="showAllAddresses"
            :addresses="addresses"
          />

          <UFormField
            v-if="model.type === 'enum'"
            label="Options"
            required
            :ui="{ container: 'grid gap-2' }"
          >
            <EnumOptionsField v-model="enumOptions" />
          </UFormField>

          <template v-if="model.type === 'open-shock-shocker'">
            <UAlert
              v-if="!openShockAvailable"
              color="warning"
              variant="subtle"
              icon="lucide:shield-off"
              title="OpenShock unavailable"
              description="No valid OpenShock API key is configured. Add one in Settings - until then this control shows as unavailable to everyone, including you."
            />
            <OpenShockFields
              v-model:mode="model.mode"
              v-model:shockers="model.shockers"
              v-model:intensity="model.intensity"
              v-model:duration="model.duration"
              v-model:cooldown="model.cooldown"
              :open-shock-mode="openShockMode"
              :shocker-list="shockerList"
            />
          </template>

          <template v-if="model.type === 'intiface-toy'">
            <UAlert
              v-if="!intifaceAvailable"
              color="warning"
              variant="subtle"
              icon="lucide:shield-off"
              title="Intiface unavailable"
              description="Not connected to an Intiface Engine. Enable and configure it in Settings - until then this control shows as unavailable to everyone, including you."
            />
            <IntifaceFields
              v-model:actuators="intifaceActuators"
              :devices="intifaceDevices"
            />
          </template>

          <template v-if="model.type === 'intiface-pattern'">
            <UAlert
              v-if="!intifaceAvailable"
              color="warning"
              variant="subtle"
              icon="lucide:shield-off"
              title="Intiface unavailable"
              description="Not connected to an Intiface Engine. Enable and configure it in Settings - until then this control shows as unavailable to everyone, including you."
            />
            <IntifaceFields
              v-model:actuators="intifacePatternActuators"
              :devices="intifaceDevices"
            />
            <IntifacePatternFields v-model:allowed-patterns="intifaceAllowedPatterns" />
          </template>

          <template v-if="model.type === 'preset'">
            <PresetField v-model="presetControlId" />
          </template>
        </UForm>
        <div class="flex flex-col justify-between gap-2">
          <Control
            class="w-55"
            :control="model as ControlType"
            :unavailable="
              (model.type === 'open-shock-shocker' && !openShockAvailable) ||
                ((model.type === 'intiface-toy' || model.type === 'intiface-pattern') &&
                  intifaceControlUnavailable)
            "
            @command="handleCommand(model as ControlType, $event)"
          />
          <div class="flex justify-end gap-2">
            <UButton
              variant="subtle"
              color="neutral"
              @click="open = false"
            >
              Cancel
            </UButton>
            <UButton @click="submit">
              Save
            </UButton>
          </div>
        </div>
      </UCard>
    </template>
  </UModal>
</template>

<style scoped></style>
