<script setup lang="ts">
import type { Rich, Sentence } from '@renderer/utils/automationSentence'
import { seconds } from '@renderer/utils/automationSentence'
import { h } from 'vue'

// An automation as one plain sentence, with names and values in bold (see automationSentence.ts).
defineProps<{ sentence: Sentence }>()

const RichText = (props: { parts: Rich }) =>
  props.parts.map((part) => (part.strong ? h('b', { class: 'font-medium text-highlighted' }, part.text) : part.text))
RichText.props = ['parts']
</script>

<template>
  <span>
    {{ sentence.lead }} <RichText :parts="sentence.when" />,
    <template
      v-for="(step, index) in sentence.steps"
      :key="index"
    >
      <template v-if="index">
        then{{ step.delayMs ? ` after ${seconds(step.delayMs)}` : '' }}
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
