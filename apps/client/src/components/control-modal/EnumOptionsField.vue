<script setup lang="ts">
import IconSelectMenu from '@renderer/components/IconSelectMenu.vue'
import type { EnumControl } from '@vrc-osc-toolkit/shared-ui'
import { computed } from 'vue'

const options = defineModel<EnumControl['options']>({ required: true })

const sortedOptions = computed(() => [...options.value].sort((a, b) => a.value - b.value))

const addOption = (): void => {
  const nextValue = Math.max(0, ...options.value.map((option) => option.value)) + 1

  options.value = [...options.value, { name: '', value: nextValue, icon: '' }]
}

const removeOption = (value: number): void => {
  options.value = options.value.filter((option) => option.value !== value)
}
</script>

<template>
  <div
    v-for="option in sortedOptions"
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
      @click="removeOption(option.value)"
    />
  </div>
  <UButton
    icon="i-lucide:plus"
    color="primary"
    size="sm"
    variant="outline"
    block
    class="grow-0"
    @click="addOption"
  />
</template>

<style scoped></style>
