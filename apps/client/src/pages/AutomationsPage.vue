<script setup lang="ts">
import AutomationIdeas from '@renderer/components/automations/AutomationIdeas.vue'
import AutomationSentence from '@renderer/components/automations/AutomationSentence.vue'
import ConfirmDialog from '@renderer/components/ui/ConfirmDialog.vue'
import EmptyState from '@renderer/components/ui/EmptyState.vue'
import FormSection from '@renderer/components/ui/FormSection.vue'
import { useAddons } from '@renderer/composables/useAddons'
import { useAutomations } from '@renderer/composables/useAutomations'
import { useAvatarDetails } from '@renderer/composables/useAvatarDetails'
import { useBoards } from '@renderer/composables/useBoards'
import { useControls } from '@renderer/composables/useControls'
import type { Automation } from '@renderer/lib/tauri-bridge'
import { easyExtras, plainSentence, sentence } from '@renderer/utils/automationSentence'
import { timeAgo } from '@renderer/utils/time'
import { useNow } from '@vueuse/core'
import { computed, ref } from 'vue'

// Every automation: when something happens, do something. Avatar-parameter automations only run
// while their avatar is worn, so the current avatar's come first.
const { automations, running, runs, setEnabled, describeContext, duplicate, importJson } = useAutomations()
const { controls } = useControls()
const { boards } = useBoards()
const { offAddonForAutomationKind, offAddonForControlType } = useAddons()
const toast = useToast()
// Re-renders the "x min ago" now and then.
const now = useNow({ interval: 30_000 })

const lastRan = (id: string): string => {
  const run = runs.value[id]
  const ran = run?.count ? `Ran ${run.count === 1 ? 'once' : `${run.count} times`} · last ${timeAgo(run.at, now.value)}` : "Hasn't run yet"
  return run?.skipped ? `${ran} · ${run.skipped}` : ran
}

const { avatarDetails } = useAvatarDetails()

// Whose an automation is, or null when it runs on any avatar.
const avatarOf = (automation: Automation): { id: string; name: string } | null => {
  const { when } = automation
  if (when.kind === 'avatar-changed') return when.avatarId ? { id: when.avatarId, name: when.avatarName ?? 'Avatar' } : null
  if (when.kind === 'avatar-parameter') return when.anyAvatar ? null : { id: when.avatarId, name: when.avatarName }
  return null
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

// Steps that can't run as things are now: an add-on that's off, a control the worn avatar doesn't
// have, an output or an automation that was deleted.
const problems = (automation: Automation): string[] => {
  const worn = !avatarOf(automation) || avatarOf(automation)?.id === avatarDetails.value?.id
  const hasControl = (id: string): boolean => controls.value.some((group) => group.controls.some((c) => c.id === id))
  return automation.then.flatMap((step, i) => {
    const n = `Step ${i + 1}`
    const addon = offAddonForAutomationKind(step.kind) ?? (step.kind === 'control' ? offAddonForControlType(step.controlType) : undefined)
    if (addon) return [`${n}: ${addon.name} is off`]
    if (step.kind === 'control' && worn && step.controlId && !hasControl(step.controlId)) return [`${n}: this avatar doesn't have its control`]
    if (step.kind === 'board-output' && !boards.value.find((b) => b.id === step.boardId)?.config?.some((o) => o.pin === step.pin)) {
      return [`${n}: its output was removed`]
    }
    if (step.kind === 'automation-enabled' && !automations.value.some((a) => a.id === step.automationId)) return [`${n}: that automation was deleted`]
    return []
  })
}
const problemText = (automation: Automation): string => {
  const count = problems(automation).length
  return count === 1 ? "A step can't run" : `${count} steps can't run`
}

const search = ref('')
const matches = (automation: Automation): boolean => {
  const query = search.value.trim().toLowerCase()
  return !query || `${automation.name} ${plainSentence(sentence(automation, describeContext.value))}`.toLowerCase().includes(query)
}

const menu = (automation: Automation) => [
  { label: 'Duplicate', icon: 'i-lucide-copy', onSelect: () => void duplicate(automation) },
  {
    label: 'Copy as JSON',
    icon: 'i-lucide-clipboard-copy',
    onSelect: async () => {
      await navigator.clipboard.writeText(JSON.stringify(automation, null, 2))
      toast.add({ title: 'Copied', description: 'Paste it in Import, here or on another computer.', icon: 'i-lucide-clipboard-check' })
    }
  }
]

// Import: pasted JSON from Copy as JSON.
const importing = ref(false)
const importText = ref('')
const importError = ref('')
const runImport = async (): Promise<void> => {
  try {
    const count = await importJson(importText.value)
    importText.value = ''
    importError.value = ''
    toast.add({ title: `Imported ${count === 1 ? 'one automation' : `${count} automations`}`, description: 'They stay off until you turn them on.', icon: 'i-lucide-download' })
  } catch (error) {
    importError.value = error instanceof Error ? error.message : String(error)
    importing.value = true
  }
}

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
].map((group) => ({ ...group, automations: group.automations.filter(matches) })))

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
            icon="i-lucide-download"
            color="neutral"
            variant="subtle"
            @click="importing = true"
          >
            Import
          </UButton>
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

    <AutomationIdeas class="mb-3.5" />

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
      class="grid gap-3.5"
    >
      <UInput
        v-if="automations.length > 4"
        v-model="search"
        icon="i-lucide-search"
        placeholder="Search by name, trigger or step"
        class="w-full"
      />
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
                  <span
                    v-if="problems(automation).length"
                    class="shrink-0 rounded-full bg-warning/15 px-2 py-0.5 text-xs font-medium text-warning"
                    :title="problems(automation).join('. ')"
                  >{{ problemText(automation) }}</span>
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
              <UDropdownMenu :items="menu(automation)">
                <UButton
                  icon="i-lucide-ellipsis"
                  color="neutral"
                  variant="ghost"
                  :aria-label="`${automation.name} actions`"
                />
              </UDropdownMenu>
            </div>
          </div>
        </FormSection>
      </template>
    </div>

    <ConfirmDialog
      v-model:open="importing"
      title="Import automations"
      confirm-text="Import"
      confirm-icon="i-lucide-download"
      :danger="false"
      @confirm="runImport"
    >
      Paste what Copy as JSON gave you. Each one is added as a new automation, turned off.
      <template #fields>
        <UFormField :error="importError || false">
          <UTextarea
            v-model="importText"
            :rows="8"
            autoresize
            class="w-full font-mono"
          />
        </UFormField>
      </template>
    </ConfirmDialog>
  </PageLayout>
</template>
