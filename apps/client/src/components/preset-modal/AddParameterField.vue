<script setup lang="ts">
import type { SelectMenuItem } from '@nuxt/ui/components/SelectMenu.vue'
import AddressListToggle from '@renderer/components/control-modal/AddressListToggle.vue'
import { usePresets } from '@renderer/composables/usePresets'
import { isNoisyAddress } from '@renderer/utils/addressFilter'
import type { PresetParameter } from '@vrc-osc-toolkit/shared-ui'
import { computed, ref } from 'vue'

const parameters = defineModel<PresetParameter[]>('parameters', { required: true })

const { filterableParameters } = usePresets()

const showAll = ref(false)
const selected = ref<string>()

const includedAddresses = computed(() => new Set(parameters.value.map((p) => p.address)))

const items = computed<SelectMenuItem[]>(() =>
  filterableParameters.value
    .filter((param) => !includedAddresses.value.has(param.address))
    .filter((param) => showAll.value || (!param.excluded && !isNoisyAddress(param.name)))
    .map((param) => ({
      label: param.captured ? param.name : `${param.name} (not seen yet)`,
      value: param.address
    }))
)

const add = (): void => {
  const param = filterableParameters.value.find((p) => p.address === selected.value)

  if (!param) return

  parameters.value = [
    ...parameters.value,
    { address: param.address, name: param.name, kind: param.kind, value: param.value ?? false }
  ]

  selected.value = undefined
}
</script>

<template>
  <UFormField label="Add Parameter">
    <div class="flex gap-2">
      <USelectMenu
        v-model="selected"
        :items="items"
        value-key="value"
        placeholder="Select a parameter"
        class="w-full grow"
        virtualize
      />
      <UButton
        icon="i-lucide-plus"
        :disabled="!selected"
        @click="add"
      />
    </div>
  </UFormField>
  <AddressListToggle v-model="showAll" />
</template>

<style scoped></style>
