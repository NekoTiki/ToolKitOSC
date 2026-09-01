<script setup lang="ts">
import type { SelectMenuItem } from '@nuxt/ui/components/SelectMenu.vue'

defineProps<{ addresses: SelectMenuItem[] }>()
const model = defineModel<string>()
</script>

<template>
  <USelectMenu
    v-model="model"
    :items="addresses"
    placeholder="Select an address"
    value-key="value"
    name="Address"
    class="w-full"
    virtualize
  />
</template>

<style scoped></style>
