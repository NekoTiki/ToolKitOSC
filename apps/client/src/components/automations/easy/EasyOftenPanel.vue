<script setup lang="ts">
import FormSection from '@renderer/components/ui/FormSection.vue'
import SegmentedControl from '@renderer/components/ui/SegmentedControl.vue'
import type { Automation } from '@renderer/lib/tauri-bridge'
import { CONDITIONS, triggerCondition } from '@renderer/utils/automations'
import { EASY_COOLDOWNS, seconds } from '@renderer/utils/automationSentence'
import { computed } from 'vue'

// Easy mode, "how often": a cooldown from a few choices, and what a new moment does while the
// last run is still going.
const automation = defineModel<Automation>({ required: true })

const cooldown = computed({
  get: () => String(automation.value.cooldownMs),
  set: (value: string) => (automation.value.cooldownMs = Number(value))
})
const cooldownItems = EASY_COOLDOWNS.map((ms) => ({ value: String(ms), label: ms ? `At most every ${seconds(ms)}` : 'Every time' }))

// Only a moment can happen again while it runs; a state just keeps going.
const moment = computed(() => CONDITIONS[triggerCondition(automation.value.when)].moment)
</script>

<template>
  <FormSection title="How often?">
    <UFormField label="Run it">
      <SegmentedControl
        v-model="cooldown"
        :items="cooldownItems"
        class="w-fit max-w-full flex-wrap"
        accent
        aria-label="Run it"
      />
    </UFormField>
    <UFormField
      v-if="moment"
      label="If it happens again before it's done"
    >
      <SegmentedControl
        v-model="automation.retrigger"
        :items="[
          { value: 'restart', label: 'Start over' },
          { value: 'ignore', label: 'Let it finish' }
        ]"
        class="w-fit max-w-full flex-wrap"
        accent
        aria-label="If it happens again before it's done"
      />
    </UFormField>
  </FormSection>
</template>
