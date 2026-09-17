<script setup lang="ts">
import type { SelectMenuItem } from '@nuxt/ui/components/SelectMenu.vue'
import { usePresets } from '@renderer/composables/usePresets'
import { computed } from 'vue'

const model = defineModel<string>()

const { presets } = usePresets()

const items = computed<SelectMenuItem[]>(() =>
  presets.value.map((preset) => ({ label: preset.name, value: preset.id }))
)
</script>

<template>
  <UFormField
    label="Preset"
    required
  >
    <USelectMenu
      v-model="model"
      :items="items"
      value-key="value"
      placeholder="Select a preset"
      class="w-full"
    />
  </UFormField>
  <UAlert
    v-if="!presets.length"
    color="warning"
    variant="subtle"
    icon="lucide:info"
    title="No presets yet"
    description="Create one first from the Presets manager (header), then attach it here."
  />
</template>

<style scoped></style>
