<script setup lang="ts">
import RichText from '@renderer/components/automations/RichText'
import type { Sentence } from '@renderer/utils/automationSentence'

// An automation as one plain sentence, with names and values in bold (see automationSentence.ts).
defineProps<{ sentence: Sentence }>()
</script>

<template>
  <span class="[&_b]:font-medium [&_b]:text-highlighted">
    {{ sentence.lead }} <RichText :parts="sentence.when" />,
    <template
      v-for="(step, index) in sentence.steps"
      :key="index"
    >
      <template v-if="index">
        then{{ step.after }}
      </template>
      <RichText :parts="step.text" />,
    </template>
    <RichText
      v-if="sentence.only"
      :parts="sentence.only"
    />
    <template v-else>any time</template>,
    <RichText :parts="sentence.often" />.
  </span>
</template>
