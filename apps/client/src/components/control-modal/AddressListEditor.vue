<script setup lang="ts">
import type { SelectMenuItem } from '@nuxt/ui/components/SelectMenu.vue'
import AddressSelect from '@renderer/components/control-modal/AddressSelect.vue'

defineProps<{ addresses: SelectMenuItem[] }>()
const list = defineModel<string[]>({ required: true })

const updateEntry = (index: number, value: string | undefined): void => {
  list.value = list.value.map((entry, i) => (i === index ? (value ?? '') : entry))
}

const addEntry = (): void => {
  list.value = [...list.value, '']
}

const removeEntry = (index: number): void => {
  list.value = list.value.filter((_, i) => i !== index)
}
</script>

<template>
  <div
    v-for="(entry, index) in list"
    :key="index"
    class="flex gap-2"
  >
    <AddressSelect
      :model-value="entry"
      :addresses="addresses"
      @update:model-value="(val) => updateEntry(index, val)"
    />
    <UButton
      icon="i-lucide:trash-2"
      color="error"
      size="sm"
      variant="outline"
      class="mt-1 grow-0"
      @click="removeEntry(index)"
    />
  </div>
  <UButton
    icon="i-lucide:plus"
    color="primary"
    size="sm"
    variant="outline"
    block
    class="grow-0"
    @click="addEntry"
  />
</template>

<style scoped></style>
