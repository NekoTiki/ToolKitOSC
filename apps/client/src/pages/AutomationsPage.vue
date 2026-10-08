<script setup lang="ts">
import EmptyState from '@renderer/components/ui/EmptyState.vue'
import FormSection from '@renderer/components/ui/FormSection.vue'
import { useAutomations } from '@renderer/composables/useAutomations'
import { useAvatarDetails } from '@renderer/composables/useAvatarDetails'
import { useBoards } from '@renderer/composables/useBoards'
import type { Automation } from '@renderer/lib/tauri-bridge'
import { describe } from '@renderer/utils/automations'
import { computed } from 'vue'

// Every automation: when something happens, do something. Avatar-parameter automations only run
// while their avatar is worn, so the current avatar's come first.
const { automations, running, setEnabled } = useAutomations()
const { avatarDetails } = useAvatarDetails()
const { boards } = useBoards()

const forThisAvatar = computed(() =>
  automations.value.filter((a) => a.when.avatarId === avatarDetails.value?.id)
)

// Grouped by avatar, so the list says whose they are.
const otherAvatars = computed(() => {
  const byAvatar = new Map<string, { id: string; name: string; automations: Automation[] }>()
  for (const automation of automations.value) {
    if (automation.when.avatarId === avatarDetails.value?.id) continue
    const id = automation.when.avatarId
    const group = byAvatar.get(id) ?? { id, name: automation.when.avatarName, automations: [] }
    group.automations.push(automation)
    byAvatar.set(id, group)
  }
  return Array.from(byAvatar.values())
})

// The current avatar first (even when it has none yet, so it's clear where new ones go).
const groups = computed(() => [
  ...(avatarDetails.value
    ? [
        {
          key: avatarDetails.value.id,
          title: avatarDetails.value.name,
          description: 'The avatar you are wearing.',
          automations: forThisAvatar.value
        }
      ]
    : []),
  ...otherAvatars.value.map((group) => ({
    key: group.id,
    title: group.name,
    description: 'Only runs while you wear this avatar.',
    automations: group.automations
  }))
])

const hasOutputs = computed(() => boards.value.some((b) => b.config?.length))
</script>

<template>
  <PageLayout>
    <template #header>
      <PageHeader
        title="Automations"
        meta="When something happens, do something."
      >
        <template #actions>
          <UButton
            icon="i-lucide-plus"
            to="/automations/new"
            :disabled="!avatarDetails"
          >
            New automation
          </UButton>
        </template>
      </PageHeader>
    </template>

    <EmptyState
      v-if="!automations.length"
      icon="i-lucide-workflow"
      title="No automations yet"
      :description="
        hasOutputs
          ? 'Make your avatar switch things for real: when a parameter changes, pulse, hold or dim an output on your ESP32 board.'
          : 'Automations switch outputs on your ESP32 boards when an avatar parameter changes. Add a board and its outputs first.'
      "
    >
      <template #actions>
        <UButton
          v-if="hasOutputs"
          icon="i-lucide-plus"
          to="/automations/new"
          :disabled="!avatarDetails"
        >
          New automation
        </UButton>
        <UButton
          v-else
          icon="i-lucide-cpu"
          to="/settings/esp32"
        >
          Set up an ESP32 board
        </UButton>
      </template>
    </EmptyState>

    <div
      v-else
      class="grid max-w-4xl gap-3.5"
    >
      <template
        v-for="group in groups"
        :key="group.key"
      >
        <FormSection
          :title="group.title"
          :description="group.description"
        >
          <p
            v-if="!group.automations.length"
            class="text-sm text-muted"
          >
            No automations for this avatar yet.
          </p>
          <div
            v-else
            class="grid divide-y divide-(--ui-border)"
          >
            <div
              v-for="automation in group.automations"
              :key="automation.id"
              class="flex min-h-14 items-center gap-3 py-2.5 first:pt-0 last:pb-0"
            >
              <span
                class="grid size-10 shrink-0 place-items-center rounded-md bg-(--aurora-well)"
                :class="running.has(automation.id) ? 'text-secondary' : 'text-muted'"
              >
                <UIcon
                  name="i-lucide-workflow"
                  class="size-5"
                />
              </span>
              <div class="grid min-w-0 flex-1 gap-0.5">
                <span class="flex min-w-0 items-center gap-2">
                  <b class="truncate font-medium text-highlighted">{{ automation.name }}</b>
                  <span
                    v-if="running.has(automation.id)"
                    class="shrink-0 rounded-full bg-secondary/15 px-2 py-0.5 text-xs font-medium text-secondary"
                  >Running</span>
                </span>
                <small class="truncate text-xs text-muted">{{ describe(automation, boards) }}</small>
              </div>
              <USwitch
                :model-value="automation.enabled"
                :aria-label="`${automation.name} enabled`"
                @update:model-value="setEnabled(automation.id, !!$event)"
              />
              <UButton
                icon="i-lucide-pencil"
                color="neutral"
                variant="subtle"
                :to="`/automations/${automation.id}`"
              >
                Edit
              </UButton>
            </div>
          </div>
        </FormSection>
      </template>
    </div>
  </PageLayout>
</template>
