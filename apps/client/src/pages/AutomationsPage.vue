<script setup lang="ts">
import AutomationSentence from '@renderer/components/automations/AutomationSentence.vue'
import EmptyState from '@renderer/components/ui/EmptyState.vue'
import FormSection from '@renderer/components/ui/FormSection.vue'
import { useAutomations } from '@renderer/composables/useAutomations'
import { useAvatarDetails } from '@renderer/composables/useAvatarDetails'
import type { Automation } from '@renderer/lib/tauri-bridge'
import { easyExtras, sentence } from '@renderer/utils/automationSentence'
import { timeAgo } from '@renderer/utils/time'
import { useNow } from '@vueuse/core'
import { computed } from 'vue'

// Every automation: when something happens, do something. Avatar-parameter automations only run
// while their avatar is worn, so the current avatar's come first.
const { automations, running, runs, setEnabled, describeContext } = useAutomations()
// Re-renders the "x min ago" now and then.
const now = useNow({ interval: 30_000 })

const lastRan = (id: string): string => {
  const run = runs.value[id]
  if (!run) return "Hasn't run yet"
  return `Ran ${run.count === 1 ? 'once' : `${run.count} times`} · last ${timeAgo(run.at, now.value)}`
}

const { avatarDetails } = useAvatarDetails()

// Whose an automation is, or null when it runs on any avatar.
const avatarOf = (automation: Automation): { id: string; name: string } | null => {
  const { when } = automation
  if (when.kind === 'app-event' || when.kind === 'leash') return null
  if (when.kind === 'avatar-changed') return when.avatarId ? { id: when.avatarId, name: when.avatarName ?? 'Avatar' } : null
  return when.anyAvatar ? null : { id: when.avatarId, name: when.avatarName }
}

const forThisAvatar = computed(() => automations.value.filter((a) => avatarOf(a)?.id === avatarDetails.value?.id))
const forAnyAvatar = computed(() => automations.value.filter((a) => !avatarOf(a)))

// Grouped by avatar, so the list says whose they are.
const otherAvatars = computed(() => {
  const byAvatar = new Map<string, { id: string; name: string; automations: Automation[] }>()
  for (const automation of automations.value) {
    const avatar = avatarOf(automation)
    if (!avatar || avatar.id === avatarDetails.value?.id) continue
    const group = byAvatar.get(avatar.id) ?? { ...avatar, automations: [] }
    group.automations.push(automation)
    byAvatar.set(avatar.id, group)
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
  ...(forAnyAvatar.value.length
    ? [
        {
          key: 'any',
          title: 'Any avatar',
          description: 'Runs on whichever avatar you wear.',
          automations: forAnyAvatar.value
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
      description="Make the app react to you: when one of your avatar's parameters changes, run a control or a toy, switch profile, post to the chatbox, set another parameter or drive an ESP32 output."
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
                <small class="text-sm text-muted">
                  <AutomationSentence :sentence="sentence(automation, describeContext)" />
                  <span
                    v-if="easyExtras(automation).length"
                    class="ml-1 text-warning"
                  >· {{ easyExtras(automation).length }} advanced</span>
                </small>
                <small class="text-xs text-dimmed">{{ lastRan(automation.id) }}</small>
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
