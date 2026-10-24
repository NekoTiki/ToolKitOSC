<script setup lang="ts">
import RichText from '@renderer/components/automations/RichText'
import type { Sentence } from '@renderer/utils/automationSentence'
import { seconds } from '@renderer/utils/automationSentence'

// The Easy editor's sentence: the same words as the list, with each part a button that opens its
// panel below. Parts still to fill in are dashed.
defineProps<{
  sentence: Sentence
  whenIcon: string
  whenMissing: boolean
  stepIcons: string[]
  canAddStep: boolean
  // How many settings Easy can't show (easyExtras).
  extras: number
}>()
const emit = defineEmits<{ addStep: []; advanced: [] }>()
// 'when', 'step-<index>', 'only' or 'often'.
const selected = defineModel<string>('selected', { required: true })

const part = (slot: string, kind: 'when' | 'step' | 'extra', missing = false): string[] => [
  'mx-1 inline-flex max-w-full cursor-pointer items-center gap-2 rounded-field border px-3.5 py-1 text-left align-middle leading-snug transition-colors',
  missing
    ? 'border-dashed border-accented text-muted'
    : kind === 'when'
      ? 'border-transparent bg-primary/14 text-highlighted [&_b]:font-semibold [&_b]:text-primary'
      : kind === 'step'
        ? 'border-transparent bg-secondary/13 text-highlighted [&_b]:font-semibold [&_b]:text-secondary'
        : 'border-transparent glass text-base text-muted [&_b]:font-medium [&_b]:text-highlighted',
  selected.value === slot ? 'border-(--ui-text-highlighted)! ring-3 ring-(--ui-text-highlighted)/15' : ''
]
</script>

<template>
  <div class="grid gap-3 rounded-panel glass p-5">
    <!-- Tall lines, so the parts on wrapped lines don't touch. -->
    <p
      class="m-0 text-xl text-muted"
      style="line-height: 3"
    >
      {{ sentence.lead }}
      <button
        type="button"
        :class="part('when', 'when', whenMissing)"
        :aria-pressed="selected === 'when'"
        @click="selected = 'when'"
      >
        <UIcon
          :name="whenIcon"
          class="size-5 shrink-0"
        />
        <span><RichText :parts="sentence.when" /></span>
      </button>,
      <template
        v-for="(step, index) in sentence.steps"
        :key="index"
      >
        <template v-if="index">
          then{{ step.delayMs ? ` after ${seconds(step.delayMs)}` : '' }}
        </template>
        <button
          type="button"
          :class="part(`step-${index}`, 'step', step.missing)"
          :aria-pressed="selected === `step-${index}`"
          @click="selected = `step-${index}`"
        >
          <UIcon
            :name="stepIcons[index]!"
            class="size-5 shrink-0"
          />
          <span><RichText :parts="step.text" /></span>
        </button>,
      </template>
      <span v-if="canAddStep"><button
        type="button"
        :class="part('add', 'extra', true)"
        class="text-base"
        @click="emit('addStep')"
      >
        <UIcon
          name="i-lucide-plus"
          class="size-4.5 shrink-0"
        />
        and then…
      </button>,</span>
      <button
        type="button"
        :class="part('only', 'extra')"
        :aria-pressed="selected === 'only'"
        @click="selected = 'only'"
      >
        <UIcon
          name="i-lucide-filter"
          class="size-4.5 shrink-0"
        />
        <span><RichText
          v-if="sentence.only"
          :parts="sentence.only"
        /><template v-else>any time</template></span>
      </button>,
      <button
        type="button"
        :class="part('often', 'extra')"
        :aria-pressed="selected === 'often'"
        @click="selected = 'often'"
      >
        <UIcon
          name="i-lucide-clock"
          class="size-4.5 shrink-0"
        />
        <span><RichText :parts="sentence.often" /></span>
      </button>.
    </p>
    <div v-if="extras">
      <UButton
        icon="i-lucide-sliders-horizontal"
        color="warning"
        variant="soft"
        @click="emit('advanced')"
      >
        {{ extras }} advanced setting{{ extras === 1 ? '' : 's' }}
      </UButton>
    </div>
  </div>
</template>
