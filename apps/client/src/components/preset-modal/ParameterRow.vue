<script setup lang="ts">
import { usePresets } from '@renderer/composables/usePresets'
import type { PresetParameter } from '@toolkitosc/shared-ui'
import { computed } from 'vue'

const parameter = defineModel<PresetParameter>({ required: true })

defineEmits<{ (e: 'remove'): void }>()

const { isExcluded, setExcluded } = usePresets()

const excluded = computed(() => isExcluded(parameter.value.address))

const numberValue = computed({
  get: () => Number(parameter.value.value ?? 0),
  set: (val: number) => (parameter.value.value = val)
})

const boolValue = computed({
  get: () => Boolean(parameter.value.value),
  set: (val: boolean) => (parameter.value.value = val)
})
</script>

<template>
  <div class="flex items-center gap-2">
    <div class="min-w-0 grow">
      <p class="truncate text-sm font-medium">
        {{ parameter.name }}
      </p>
      <p class="truncate text-xs text-muted">
        {{ parameter.address }}
      </p>
    </div>

    <USwitch
      v-if="parameter.kind === 'Bool'"
      v-model="boolValue"
    />
    <UInput
      v-else
      v-model.number="numberValue"
      type="number"
      :step="parameter.kind === 'Int' ? 1 : 'any'"
      class="w-28"
    />

    <UTooltip>
      <UButton
        :icon="excluded ? 'i-lucide-eye-off' : 'i-lucide-eye'"
        color="neutral"
        :variant="excluded ? 'subtle' : 'outline'"
        size="sm"
        @click="setExcluded(parameter.address, !excluded)"
      />

      <template #content>
        {{
          excluded
            ? 'Excluded avatar-wide - re-include it in future presets'
            : 'Exclude this address avatar-wide from future presets'
        }}
      </template>
    </UTooltip>

    <UButton
      icon="i-lucide-trash-2"
      color="error"
      variant="outline"
      size="sm"
      @click="$emit('remove')"
    />
  </div>
</template>

<style scoped></style>
