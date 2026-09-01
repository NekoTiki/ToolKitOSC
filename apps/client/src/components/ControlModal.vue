<script setup lang="ts">
import type { SelectMenuItem } from '@nuxt/ui/components/SelectMenu.vue'
import AddressField from '@renderer/components/control-modal/AddressField.vue'
import BooleanEnumFields from '@renderer/components/control-modal/BooleanEnumFields.vue'
import BooleanGroupFields from '@renderer/components/control-modal/BooleanGroupFields.vue'
import EnumOptionsField from '@renderer/components/control-modal/EnumOptionsField.vue'
import OpenShockFields from '@renderer/components/control-modal/OpenShockFields.vue'
import IconSelectMenu from '@renderer/components/IconSelectMenu.vue'
import { useAvatarDetails } from '@renderer/composables/useAvatarDetails'
import { getUUID, useControlModal } from '@renderer/composables/useControlModal'
import { useControls } from '@renderer/composables/useControls'
import { useOpenShock } from '@renderer/composables/useOpenShock'
import type {
  BooleanControl,
  BooleanEnumControl,
  BooleanGroupControl,
  ControlType,
  EnumControl,
  OpenShockControl,
  SliderControl,
  StepEnumControl
} from '@vrc-osc-toolkit/shared-ui'
import { Control } from '@vrc-osc-toolkit/shared-ui'
import { computed, onMounted, ref } from 'vue'

const { model, open, submit } = useControlModal()
const { avatarDetails } = useAvatarDetails()
const { handleCommand } = useControls()
const { getShockers } = useOpenShock()

type SelectMenuItemType = SelectMenuItem & { value: ControlType['type'] }
type SelectMenuItemOpenShockMode = SelectMenuItem & { value: OpenShockControl['mode'] }

const types = ref<SelectMenuItemType[]>([
  { label: 'Toggle', value: 'boolean' },
  { label: 'Toggle Group', value: 'boolean-group' },
  { label: 'Toggle Logic', value: 'boolean-enum' },
  { label: 'Enum', value: 'enum' },
  { label: 'Slider', value: 'slider' },
  { label: 'Open Shock', value: 'open-shock-shocker' }
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
  }
}

const modelType = computed({
  get: () => model.value.type,
  set: (val: ControlType['type']) => {
    model.value.type = val
    typeDefaults[val]?.(model.value)
  }
})

const showAllAddresses = ref(false)

const isFilteredAddress = (name: string): boolean => {
  if (name.startsWith('pcs/')) return true
  if (name.startsWith('WH Lollipop/')) return true
  if (name.startsWith('OGB/')) return true
  if (name.startsWith('Go/')) return true
  if (name.startsWith('FT/')) return true
  if (/VF\d+_/.test(name)) return true

  return false
}

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

      return showAllAddresses.value || !isFilteredAddress(param.name)
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

const shockerList = ref<SelectMenuItem[]>([])

onMounted(() => {
  getShockers().then((shockers) => {
    shockerList.value = shockers
      .map((shocker) => shocker.shockers)
      .flat()
      .map((shocker) => ({
        label: shocker.name,
        value: shocker.id
      }))
  })
})
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
        </UForm>
        <div class="flex flex-col justify-between gap-2">
          <Control
            class="w-55"
            :control="model as ControlType"
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
