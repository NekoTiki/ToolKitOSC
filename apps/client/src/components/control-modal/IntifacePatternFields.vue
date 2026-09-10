<script setup lang="ts">
import { BUILTIN_PATTERNS } from '@renderer/composables/useIntifacePatterns'
import type { IntifacePatternControl } from '@vrc-osc-toolkit/shared-ui'

const allowedPatterns = defineModel<IntifacePatternControl['allowedPatterns']>('allowedPatterns', {
  required: true
})

const isAllowed = (id: string): boolean =>
  allowedPatterns.value.some((pattern) => pattern.id === id)

// Only id/name/icon are kept on the control (see IntifacePatternRef in controls.ts) - the actual
// waveform (BUILTIN_PATTERNS[number].value) stays purely a client-side implementation detail this
// control never needs to carry around.
const toggle = (pattern: (typeof BUILTIN_PATTERNS)[number], checked: boolean): void => {
  if (checked) {
    if (isAllowed(pattern.id)) return

    allowedPatterns.value = [
      ...allowedPatterns.value,
      { id: pattern.id, name: pattern.name, icon: pattern.icon }
    ]
  } else {
    allowedPatterns.value = allowedPatterns.value.filter((allowed) => allowed.id !== pattern.id)
  }
}
</script>

<template>
  <UFormField
    label="Patterns"
    required
    description="Pick which patterns viewers can choose from. 'Off' is always available and isn't one of these."
  >
    <div class="flex flex-col gap-1.5">
      <!-- Explicit, per-checkbox id: see IntifaceFields.vue's actuator checkboxes for why one of
      these sitting inside a single UFormField needs its own id rather than none at all. -->
      <UCheckbox
        v-for="(pattern, index) in BUILTIN_PATTERNS"
        :id="`intiface-pattern-${index}`"
        :key="pattern.id"
        :model-value="isAllowed(pattern.id)"
        :label="pattern.name"
        @update:model-value="toggle(pattern, !!$event)"
      />
    </div>
  </UFormField>
</template>

<style scoped></style>
