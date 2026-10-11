<script setup lang="ts">
import PatternSparkline from '@renderer/components/patterns/PatternSparkline.vue'
import type { IntifacePatternDefinition } from '@renderer/composables/useIntifacePatterns'
import { allPatterns } from '@renderer/composables/useIntifacePatterns'
import type { IntifacePatternControl } from '@toolkitosc/shared-ui'
import { computed } from 'vue'

const allowedPatterns = defineModel<IntifacePatternControl['allowedPatterns']>('allowedPatterns', {
  required: true
})

// Your recorded patterns first, then the built-in ones.
const patterns = computed(() => allPatterns())

const isAllowed = (id: string): boolean =>
  allowedPatterns.value.some((pattern) => pattern.id === id)

// Only id/name/icon are kept on the control (see IntifacePatternRef in controls.ts) - the actual
// waveform (IntifacePatternDefinition.value) stays purely a client-side implementation detail this
// control never needs to carry around.
const toggle = (pattern: IntifacePatternDefinition, checked: boolean): void => {
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
    description="Pick which patterns viewers can choose from. Your patterns come first, then the built-in ones. 'Off' is always there, so anyone can stop the toy."
  >
    <template #hint>
      <ULink
        to="/patterns"
        class="text-sm text-primary"
      >
        Manage patterns →
      </ULink>
    </template>
    <div class="grid gap-2 sm:grid-cols-2">
      <!-- Explicit, per-checkbox id: see IntifaceFields.vue's actuator checkboxes for why one of
      these sitting inside a single UFormField needs its own id rather than none at all. -->
      <label
        v-for="(pattern, index) in patterns"
        :key="pattern.id"
        :for="`intiface-pattern-${index}`"
        class="flex min-h-12 cursor-pointer items-center gap-3 rounded-2xl border border-default bg-(--aurora-well) px-3"
      >
        <UCheckbox
          :id="`intiface-pattern-${index}`"
          :model-value="isAllowed(pattern.id)"
          @update:model-value="toggle(pattern, !!$event)"
        />
        <PatternSparkline
          :pattern="pattern"
          :mine="!!pattern.durationMs"
        />
        <span
          class="flex-1 truncate"
          :class="isAllowed(pattern.id) ? 'text-highlighted' : 'text-muted'"
        >{{ pattern.name }}</span>
        <UBadge
          :label="pattern.durationMs ? 'Mine' : 'Built-in'"
          :color="pattern.durationMs ? 'primary' : 'neutral'"
          variant="soft"
          size="sm"
        />
      </label>
    </div>
  </UFormField>
</template>

<style scoped></style>
