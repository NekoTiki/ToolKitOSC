<script setup lang="ts">
import type { SelectMenuItem } from '@nuxt/ui/components/SelectMenu.vue'
import { CONTROL_ICONS } from '@vrc-osc-toolkit/shared-ui'
import { computed } from 'vue'

// Grouped by category from the single shared list (see packages/shared-ui's
// constants/controlIcons.ts) - the AI control-suggestion feature reads the exact same list server
// side, so a control it picks an icon for always matches one this picker also offers.
const icons = computed<SelectMenuItem[]>(() => {
  const categories: SelectMenuItem[][] = []
  let current: SelectMenuItem[] | null = null
  let currentCategory: string | null = null

  for (const { category, label, icon } of CONTROL_ICONS) {
    if (category !== currentCategory) {
      current = [{ type: 'label', label: category }]
      categories.push(current)
      currentCategory = category
    }

    current!.push({ label, value: icon, icon })
  }

  return categories
})

const model = defineModel<string>()
</script>

<template>
  <USelectMenu
    v-model="model"
    :leading-icon="model"
    :items="icons"
    value-key="value"
    virtualize
    placeholder="Select an icon"
  >
    <template
      v-if="model"
      #trailing
    >
      <UButton
        color="neutral"
        variant="link"
        size="sm"
        icon="i-lucide-circle-x"
        aria-label="Clear input"
        @click.prevent="model = ''"
      />
    </template>
  </USelectMenu>
</template>

<style scoped></style>
