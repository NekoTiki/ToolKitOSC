<script setup lang="ts">
import { computed } from 'vue'

import { useIntifacePatternControl } from '../composables/useIntifacePatternControl'
import type { IntifacePatternRef } from '../types/controls'
import { INTIFACE_PATTERN_OFF_ID } from '../types/controls'
import ControlBase from './ControlBase.vue'

// Same chip-picker treatment as ControlEnum.vue, for the same reason (every choice tappable and
// visible at once, no dropdown) - the only real difference is the value being played back over
// time client-side (see useIntifacePatterns.ts) instead of a single static value, which this
// widget doesn't need to know or care about; it just reflects whichever pattern id is currently
// active and emits whichever one gets tapped.
const props = defineProps<{
  controlId: string
  title: string
  icon?: string
  patterns: IntifacePatternRef[]
}>()

const emit = defineEmits<{ (e: 'update:value', patternId: string): void }>()

const { controlValue } = useIntifacePatternControl()

const currentPatternId = computed(() => controlValue.value.get(props.controlId) ?? INTIFACE_PATTERN_OFF_ID)

// 'Off' isn't one of `patterns` (a control's configurable allowedPatterns) - it's always offered,
// first, regardless of what the control's author picked.
const chips = computed<IntifacePatternRef[]>(() => [
  { id: INTIFACE_PATTERN_OFF_ID, name: 'Off', icon: 'i-lucide-power-off' },
  ...props.patterns
])
</script>

<template>
  <control-base
    :title="title"
    :icon="icon"
  >
    <div class="flex flex-wrap items-center justify-center gap-1.5">
      <button
        v-for="chip in chips"
        :key="chip.id"
        type="button"
        class="flex items-center gap-1.5 rounded-full px-3 py-1.5 text-sm font-bold transition-colors"
        :class="
          chip.id === currentPatternId
            ? 'bg-primary text-inverted'
            : 'bg-elevated text-muted hover:bg-accented'
        "
        @click.stop="emit('update:value', chip.id)"
      >
        <UIcon
          v-if="chip.icon"
          :name="chip.icon"
          class="size-4"
        />
        {{ chip.name }}
      </button>
    </div>
  </control-base>
</template>

<style scoped></style>
