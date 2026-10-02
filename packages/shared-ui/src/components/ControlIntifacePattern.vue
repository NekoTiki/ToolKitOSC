<script setup lang="ts">
import { computed } from 'vue'

import { useIntifacePatternControl } from '../composables/useIntifacePatternControl'
import type { IntifacePatternRef } from '../types/controls'
import { INTIFACE_PATTERN_OFF_ID } from '../types/controls'
import ControlBase from './ControlBase.vue'
import ControlOptions from './ControlOptions.vue'

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

const options = computed(() =>
  chips.value.map((chip) => ({ key: chip.id, name: chip.name, icon: chip.icon, selected: chip.id === currentPatternId.value }))
)
</script>

<template>
  <control-base
    :title="title"
    :icon="icon"
    kind="value"
    fill
  >
    <template #badge>
      <span
        class="grid size-5 shrink-0 place-items-center rounded-full bg-(--aurora-well) text-muted"
        title="Intiface"
      >
        <UIcon
          name="mdi:vibrate"
          class="size-3"
        />
      </span>
    </template>
    <ControlOptions
      :options="options"
      pill
      @select="emit('update:value', chips[$event]!.id)"
    />
  </control-base>
</template>
