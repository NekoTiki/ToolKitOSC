<script setup lang="ts">
import type { SelectMenuItem } from '@nuxt/ui/components/SelectMenu.vue'
import AddressListToggle from '@renderer/components/control-modal/AddressListToggle.vue'
import AddressSelect from '@renderer/components/control-modal/AddressSelect.vue'

defineProps<{ addresses: SelectMenuItem[] }>()
const address = defineModel<string>('address')
const showAll = defineModel<boolean>('showAll', { required: true })
</script>

<template>
  <UFormField
    label="Address"
    required
  >
    <AddressSelect
      v-model="address"
      :addresses="addresses"
    />
  </UFormField>
  <AddressListToggle v-model="showAll" />
</template>

<style scoped></style>
