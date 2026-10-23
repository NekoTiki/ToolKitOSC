<script setup lang="ts">
import ChoiceCard from '@renderer/components/ui/ChoiceCard.vue'
import { useAutomationIdeas } from '@renderer/composables/useAutomationIdeas'
import { useTemplateRef } from 'vue'
import { useRouter } from 'vue-router'

// "Start from an idea": a row of ready-made automations that scrolls sideways, with large
// previous/next buttons since a VR pointer can't drag it.
const { ideas } = useAutomationIdeas()
const router = useRouter()
const row = useTemplateRef<HTMLElement>('row')

const scroll = (direction: 1 | -1): void => {
  row.value?.scrollBy({ left: direction * row.value.clientWidth * 0.8, behavior: 'smooth' })
}
</script>

<template>
  <section
    v-if="ideas.length"
    class="grid gap-2.5"
    aria-label="Start from an idea"
  >
    <div class="flex items-center gap-2">
      <div class="grid flex-1 gap-0.5">
        <h2 class="text-[15px] font-semibold text-highlighted">
          Start from an idea
        </h2>
        <p class="text-[12.5px] text-muted">
          Pick one, then change what you like.
        </p>
      </div>
      <UButton
        icon="i-lucide-chevron-left"
        color="neutral"
        variant="subtle"
        aria-label="Previous ideas"
        @click="scroll(-1)"
      />
      <UButton
        icon="i-lucide-chevron-right"
        color="neutral"
        variant="subtle"
        aria-label="More ideas"
        @click="scroll(1)"
      />
    </div>
    <div
      ref="row"
      class="flex snap-x snap-mandatory gap-2.5 overflow-x-auto pb-1"
    >
      <ChoiceCard
        v-for="idea in ideas"
        :key="idea.id"
        :title="idea.title"
        :description="idea.description"
        :icon="idea.icons[0]"
        class="w-64 shrink-0 snap-start"
        @click="router.push({ path: '/automations/new', query: { idea: idea.id } })"
      >
        <template #aside>
          <UIcon
            name="i-lucide-arrow-right"
            class="size-3.5 shrink-0 text-dimmed"
          />
          <UIcon
            :name="idea.icons[1]"
            class="size-4.5 shrink-0 text-secondary"
          />
        </template>
      </ChoiceCard>
    </div>
  </section>
</template>
