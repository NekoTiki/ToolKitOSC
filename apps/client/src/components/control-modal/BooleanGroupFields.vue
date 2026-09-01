<script setup lang="ts">
import type { SelectMenuItem } from '@nuxt/ui/components/SelectMenu.vue'
import AddressListEditor from '@renderer/components/control-modal/AddressListEditor.vue'
import AddressListToggle from '@renderer/components/control-modal/AddressListToggle.vue'
import type { BooleanGroupControl } from '@vrc-osc-toolkit/shared-ui'
import { computed } from 'vue'

defineProps<{ addresses: SelectMenuItem[] }>()
const inputs = defineModel<BooleanGroupControl['inputs']>('inputs', { required: true })
const showAll = defineModel<boolean>('showAll', { required: true })

// BooleanGroupControl inputs also carry an (unused, never surfaced in this form) optional `name`
// - only `inputAddress` is edited here, so the editor works with a plain string[] and this proxy
// re-wraps it back into the `{ inputAddress }[]` shape the control actually stores.
const addressList = computed<string[]>({
  get: () => inputs.value.map((input) => input.inputAddress),
  set: (val) => {
    inputs.value = val.map((inputAddress) => ({ inputAddress }))
  }
})
</script>

<template>
  <UFormField
    label="Addresses"
    :ui="{ container: 'grid gap-2' }"
    required
  >
    <AddressListEditor
      v-model="addressList"
      :addresses="addresses"
    />
  </UFormField>
  <AddressListToggle v-model="showAll" />
</template>

<style scoped></style>
