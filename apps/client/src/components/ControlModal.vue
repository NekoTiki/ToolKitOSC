<script setup lang="ts">
import type { SelectMenuItem } from '@nuxt/ui/components/SelectMenu.vue'
import IconSelectMenu from '@renderer/components/IconSelectMenu.vue'
import { useAvatarDetails } from '@renderer/composables/useAvatarDetails'
import { getUUID, useControlModal } from '@renderer/composables/useControlModal'
import { useControls } from '@renderer/composables/useControls'
import { useOpenShock } from '@renderer/composables/useOpenShock'
import type { ControlType, OpenShockControl } from '@vrc-osc-toolkit/shared-ui'
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

const modelType = computed({
  get: () => model.value.type,
  set: (val: ControlType['type']) => {
    model.value.type = val
    if (model.value.type === 'enum') model.value.options = [{ name: '', value: 0, icon: '' }]
    if (model.value.type === 'boolean-group') model.value.inputs = [{ inputAddress: '' }]
    if (model.value.type === 'boolean-enum')
      model.value.inputs = [
        {
          id: getUUID(),
          name: '',
          icon: '',
          inputAddress: { true: [''], false: [''] }
        }
      ]
    if (
      model.value.type === 'boolean' ||
      model.value.type === 'enum' ||
      model.value.type === 'slider' ||
      model.value.type === 'step-enum'
    ) {
      model.value.inputAddress = ''
    }
    if (model.value.type === 'open-shock-shocker') {
      model.value.mode = 'Shock'
      model.value.shockers = []
      model.value.intensity = { min: 0, max: 100 }
      model.value.duration = { min: 300, max: 1000 }
      model.value.cooldown = 1000
      model.value.animationDuration = 3000
    }
  }
})

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

      return !param.name.startsWith('FT/') && typeCheck
    })
    .map((param) => ({
      label: param.name,
      value: param.input?.address
      // TODO: UI bugged as of version 2.1.0 of Nuxt UI
      // description: `Current Value: ${get<unknown>(param.input?.address || '', false)}`
    }))
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
          <UFormField
            v-if="
              model.type === 'boolean' ||
                model.type === 'enum' ||
                model.type === 'slider' ||
                model.type === 'step-enum'
            "
            label="Address"
            required
          >
            <USelectMenu
              v-model="model.inputAddress"
              :items="addresses"
              placeholder="Select an address"
              value-key="value"
              name="Address"
              class="w-full"
              virtualize
            />
          </UFormField>
          <UCheckbox
            v-if="model.type === 'boolean'"
            v-model="model.reverse"
            label="Reverse Mode"
            description="(On = Off, Off = On)"
          />

          <UFormField
            v-if="model.type === 'boolean-group'"
            label="Addresses"
            :ui="{ container: 'grid gap-2' }"
            required
          >
            <div
              v-for="option in model.inputs"
              :key="option.inputAddress"
              class="flex gap-2"
            >
              <USelectMenu
                v-model="option.inputAddress"
                :items="addresses"
                placeholder="Select an address"
                value-key="value"
                name="Address"
                class="w-full"
                virtualize
              />
              <UButton
                icon="i-lucide:trash-2"
                color="error"
                size="sm"
                variant="outline"
                class="mt-1 grow-0"
                @click="
                  model.inputs = model.inputs?.filter((o) => o.inputAddress !== option.inputAddress)
                "
              />
            </div>
            <UButton
              icon="i-lucide:plus"
              color="primary"
              size="sm"
              variant="outline"
              block
              class="grow-0"
              @click="model.inputs?.push({ inputAddress: '' })"
            />
          </UFormField>

          <div
            v-if="model.type === 'boolean-enum'"
            class="flex flex-col gap-4"
          >
            <UFormField
              v-for="(option, index) in model.inputs"
              :key="option.id"
              :ui="{ container: 'flex flex-col gap-2' }"
              :label="`Option ${index}`"
              required
            >
              <div class="flex gap-2">
                <IconSelectMenu
                  v-model="option.icon"
                  class="min-w-42"
                  @keyup.backspace="option.icon = ''"
                />
                <UInput
                  v-model="option.name"
                  class="grow"
                  placeholder="Name"
                />
                <UButton
                  icon="i-lucide:trash-2"
                  color="error"
                  size="sm"
                  variant="outline"
                  class="mt-1 grow-0"
                  @click="
                    model.inputs = model.inputs?.filter(
                      (o) => o.inputAddress !== option.inputAddress
                    )
                  "
                />
              </div>
              <div class="grid grid-cols-2 gap-2">
                <UFormField
                  :ui="{ container: 'flex flex-col gap-2' }"
                  label="True"
                >
                  <div
                    v-for="(_optTrue, index) in option.inputAddress.true"
                    :key="index"
                    class="flex gap-2"
                  >
                    <USelectMenu
                      v-model="option.inputAddress.true[index]"
                      :items="addresses"
                      placeholder="Select an address"
                      value-key="value"
                      name="Address"
                      class="w-full"
                      virtualize
                    />
                    <UButton
                      icon="i-lucide:trash-2"
                      color="error"
                      size="sm"
                      variant="outline"
                      class="mt-1 grow-0"
                      @click="option.inputAddress.true.splice(index, 1)"
                    />
                  </div>
                  <UButton
                    icon="i-lucide:plus"
                    color="primary"
                    size="sm"
                    variant="outline"
                    block
                    class="grow-0"
                    @click="option.inputAddress.true.push('')"
                  />
                </UFormField>
                <UFormField
                  :ui="{ container: 'flex flex-col gap-2' }"
                  label="False"
                >
                  <div
                    v-for="(_optFalse, index) in option.inputAddress.false"
                    :key="index"
                    class="flex gap-2"
                  >
                    <USelectMenu
                      v-model="option.inputAddress.false[index]"
                      :items="addresses"
                      placeholder="Select an address"
                      value-key="value"
                      name="Address"
                      class="w-full"
                      virtualize
                    />
                    <UButton
                      icon="i-lucide:trash-2"
                      color="error"
                      size="sm"
                      variant="outline"
                      class="mt-1 grow-0"
                      @click="option.inputAddress.false.splice(index, 1)"
                    />
                  </div>
                  <UButton
                    icon="i-lucide:plus"
                    color="primary"
                    size="sm"
                    variant="outline"
                    block
                    class="grow-0"
                    @click="option.inputAddress.false.push('')"
                  />
                </UFormField>
              </div>
            </UFormField>
            <UButton
              icon="i-lucide:plus"
              color="primary"
              size="sm"
              variant="outline"
              block
              class="grow-0"
              @click="
                model.inputs?.push({
                  id: getUUID(),
                  name: '',
                  icon: '',
                  inputAddress: { true: [''], false: [''] }
                })
              "
            />
          </div>

          <UFormField
            v-if="model.type === 'enum'"
            label="Options"
            required
            :ui="{ container: 'grid gap-2' }"
          >
            <div
              v-for="option in model.options?.sort((a, b) => a.value - b.value)"
              :key="option.value"
              class="flex gap-2"
            >
              <uCard :ui="{ root: 'w-10 flex justify-center items-center ', body: 'p-0 sm:p-0' }">
                {{ option.value }}
              </uCard>
              <IconSelectMenu
                v-model="option.icon"
                class="min-w-42"
                @keyup.backspace="option.icon = ''"
              />
              <UInput
                v-model="option.name"
                class="grow"
                placeholder="Name"
              />
              <UButton
                icon="i-lucide:trash-2"
                color="error"
                size="sm"
                variant="outline"
                class="grow-0"
                @click="model.options = model.options?.filter((o) => o.value !== option.value)"
              />
            </div>
            <UButton
              icon="i-lucide:plus"
              color="primary"
              size="sm"
              variant="outline"
              block
              class="grow-0"
              @click="
                model.options?.push({
                  name: '',
                  value: Math.max(...(model.options?.map((o) => o.value) || [0])) + 1,
                  icon: ''
                })
              "
            />
          </UFormField>

          <UFormField
            v-if="model.type === 'open-shock-shocker'"
            label="Shock Mode"
            required
          >
            <USelect
              v-model="model.mode"
              :items="openShockMode"
              placeholder="Select Shockers"
              value-key="value"
              class="w-full"
              virtualize
            />
          </UFormField>
          <UFormField
            v-if="model.type === 'open-shock-shocker'"
            label="Shockers"
            required
          >
            <USelect
              v-model="model.shockers"
              :items="shockerList"
              placeholder="Select Shockers"
              value-key="value"
              class="w-full"
              multiple
              virtualize
            />
          </UFormField>
          <UFormField
            v-if="model.type === 'open-shock-shocker' && model.intensity"
            label="Intensity"
            required
          >
            <div class="grid grid-cols-2 gap-4">
              <div class="flex flex-col gap-2">
                Min: {{ model.intensity.min }}
                <USlider
                  v-model="model.intensity.min"
                  type="number"
                  :min="0"
                  :max="100"
                  class="w-full"
                />
              </div>
              <div class="flex flex-col gap-2">
                Max: {{ model.intensity.max }}
                <USlider
                  v-model="model.intensity.max"
                  type="number"
                  :min="0"
                  :max="100"
                  class="w-full"
                />
              </div>
            </div>
          </UFormField>
          <UFormField
            v-if="model.type === 'open-shock-shocker' && model.duration"
            label="Duration"
            required
          >
            <div class="grid grid-cols-2 gap-4">
              <div class="flex flex-col gap-2">
                Min: {{ model.duration.min / 1000 }}s
                <USlider
                  v-model="model.duration.min"
                  type="number"
                  :min="300"
                  :max="10000"
                  :step="100"
                  class="w-full"
                />
              </div>
              <div class="flex flex-col gap-2">
                Max: {{ model.duration.max / 1000 }}s
                <USlider
                  v-model="model.duration.max"
                  type="number"
                  :min="300"
                  :max="10000"
                  :step="100"
                  class="w-full"
                />
              </div>
            </div>
          </UFormField>
          <UFormField
            v-if="model.type === 'open-shock-shocker' && typeof model.cooldown === 'number'"
            label="CoolDown"
            required
          >
            <div class="grid gap-4">
              <div class="flex flex-col gap-2">
                Min: {{ model.cooldown / 1000 }}s
                <USlider
                  v-model="model.cooldown"
                  type="number"
                  :min="0"
                  :max="120000"
                  :step="100"
                  class="w-full"
                />
              </div>
            </div>
          </UFormField>
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
