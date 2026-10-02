<script setup lang="ts">
import type { SelectMenuItem } from '@nuxt/ui/components/SelectMenu.vue'
import AddressListEditor from '@renderer/components/control-modal/AddressListEditor.vue'
import AddressListToggle from '@renderer/components/control-modal/AddressListToggle.vue'
import IconSelectMenu from '@renderer/components/IconSelectMenu.vue'
import { getUUID } from '@renderer/utils/uuid'
import type { BooleanEnumControl } from '@toolkitosc/shared-ui'

defineProps<{ addresses: SelectMenuItem[] }>()
const inputs = defineModel<BooleanEnumControl['inputs']>('inputs', { required: true })
const showAll = defineModel<boolean>('showAll', { required: true })

const addOption = (): void => {
  inputs.value = [
    ...inputs.value,
    { id: getUUID(), name: '', icon: '', inputAddress: { true: [''], false: [''] } }
  ]
}

const removeOption = (id: string): void => {
  inputs.value = inputs.value.filter((option) => option.id !== id)
}
</script>

<template>
  <div class="flex flex-col gap-4">
    <UFormField
      v-for="(option, index) in inputs"
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
          @click="removeOption(option.id)"
        />
      </div>
      <div class="grid grid-cols-2 gap-2">
        <UFormField
          :ui="{ container: 'flex flex-col gap-2' }"
          label="True"
        >
          <AddressListEditor
            v-model="option.inputAddress.true"
            :addresses="addresses"
          />
        </UFormField>
        <UFormField
          :ui="{ container: 'flex flex-col gap-2' }"
          label="False"
        >
          <AddressListEditor
            v-model="option.inputAddress.false"
            :addresses="addresses"
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
      @click="addOption"
    />
  </div>
  <AddressListToggle v-model="showAll" />
</template>

<style scoped></style>
