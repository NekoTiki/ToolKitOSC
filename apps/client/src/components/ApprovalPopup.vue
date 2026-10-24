<script setup lang="ts">
import ConfirmDialog from '@renderer/components/ui/ConfirmDialog.vue'
import { useAutomations } from '@renderer/composables/useAutomations'
import { useControls } from '@renderer/composables/useControls'
import { useLockedControls } from '@renderer/composables/useLockedControls'
import { useMcpApprovals } from '@renderer/composables/useMcpBridge'
import { useSavedPatterns } from '@renderer/composables/useSavedPatterns'
import { computed } from 'vue'

// An AI assistant wants to use a tool that's on Ask in Settings > Assistants. Shown over whatever
// page you're on, one at a time. Left alone for a minute, Rust counts it as Deny.
const { approvals, answerApproval } = useMcpApprovals()
const { getControl } = useControls()
const { automations } = useAutomations()
const { getLockedControlGroup } = useLockedControls()
const { getSavedPattern } = useSavedPatterns()

const current = computed(() => approvals.value[0] ?? null)
const open = computed(() => !!current.value)
const args = computed(() => current.value?.arguments ?? {})

const details = computed(() => (current.value?.arguments ? JSON.stringify(current.value.arguments, null, 2) : null))

const show = (value: unknown): string => {
  if (value === true) return 'On'
  if (value === false) return 'Off'
  if (value === undefined || value === null) return 'nothing'

  return typeof value === 'string' || typeof value === 'number' ? String(value) : JSON.stringify(value)
}

// Names instead of ids, falling back to the id when it's not found.
const controlName = (): string => getControl(String(args.value.groupId), String(args.value.controlId))?.name ?? String(args.value.controlId)
const automationName = (id: unknown): string => automations.value.find((automation) => automation.id === id)?.name ?? String(id)
const profileName = (id: unknown): string => getLockedControlGroup(String(id))?.name ?? String(id)
const patternName = (id: unknown): string => getSavedPattern(String(id))?.name ?? String(id)

const draftName = (): string => {
  const { kind, id, groupId } = args.value
  if (kind === 'control') return getControl(String(groupId), String(id))?.name ?? String(id)
  if (kind === 'profile') return profileName(id)
  if (kind === 'automation') return automationName(id)
  if (kind === 'pattern') return patternName(id)

  return String(id)
}

// What the call does, as "<before> <bold name> <after>".
const summary = computed((): { before: string; name: string; after?: string } => {
  const a = args.value
  switch (current.value?.tool) {
    case 'run_control':
      return { before: 'Use', name: controlName(), after: a.value === undefined || a.value === null ? undefined : `, set to ${show(a.value)}` }
    case 'fire_shocker':
      return { before: 'Fire the shocker', name: controlName() }
    case 'send_chatbox':
      return { before: 'Post to your chatbox:', name: `“${String(a.text)}”` }
    case 'set_parameter':
      return { before: 'Set the parameter', name: String(a.name), after: ` to ${show(a.value)}` }
    case 'switch_profile':
      return { before: 'Switch to the profile', name: profileName(a.id) }
    case 'save_automation':
      return { before: 'Save the automation', name: String(a.name), after: ' as a draft, turned off' }
    case 'test_automation':
      return { before: 'Test the draft automation', name: automationName(a.id) }
    case 'save_pattern':
      return { before: 'Save the toy pattern', name: String(a.name), after: ' as a draft' }
    case 'save_profile':
      return { before: 'Save the profile', name: String(a.name), after: ' as a draft' }
    case 'create_controls': {
      const count = Array.isArray(a.controls) ? a.controls.length : 0
      return { before: 'Add', name: `${count} draft control${count === 1 ? '' : 's'}` }
    }
    case 'update_control':
      return { before: 'Change', name: controlName() }
    case 'delete_draft':
      return { before: `Delete your draft ${String(a.kind)}`, name: draftName() }
    default:
      return { before: 'It wants to use', name: current.value?.tool ?? '' }
  }
})

// Below the sentence: the new controls' names, or each change to a control as now → after.
const lines = computed((): string[] => {
  const a = args.value
  if (current.value?.tool === 'create_controls' && Array.isArray(a.controls)) {
    return a.controls.map((control) => String((control as Record<string, unknown>)?.name ?? 'Unnamed'))
  }
  if (current.value?.tool === 'update_control' && a.changes && typeof a.changes === 'object') {
    const control = getControl(String(a.groupId), String(a.controlId)) as Record<string, unknown> | undefined
    return Object.entries(a.changes).map(([key, value]) => `${key}: ${show(control?.[key])} → ${show(value)}`)
  }

  return []
})

const answer = (value: 'allow' | 'allow-10-min' | 'deny'): void => {
  if (current.value) answerApproval(current.value.id, value)
}
</script>

<template>
  <ConfirmDialog
    :open="open"
    :title="`${current?.client ?? 'An assistant'} asks`"
    confirm-text="Allow"
    cancel-text="Deny"
    confirm-icon="i-lucide-check"
    :danger="current?.tool === 'fire_shocker'"
    @confirm="answer('allow')"
    @cancel="answer('deny')"
    @update:open="(value) => !value && answer('deny')"
  >
    <p>
      {{ summary.before }} <strong>{{ summary.name }}</strong>{{ summary.after ?? '' }}
    </p>
    <template #fields>
      <ul
        v-if="lines.length"
        class="max-h-60 list-disc overflow-auto ps-5 text-sm text-default"
      >
        <li
          v-for="line in lines"
          :key="line"
        >
          {{ line }}
        </li>
      </ul>
      <UAccordion
        v-if="details"
        :items="[{ label: 'Show details', slot: 'json' }]"
        :ui="{ trigger: 'min-h-10 text-sm text-muted' }"
      >
        <template #json-body>
          <pre class="max-h-60 overflow-auto rounded-2xl bg-(--aurora-well) p-3 font-mono text-xs whitespace-pre-wrap text-default">{{ details }}</pre>
        </template>
      </UAccordion>
      <UButton
        label="Allow this tool for 10 minutes"
        icon="i-lucide-timer"
        color="neutral"
        variant="outline"
        size="lg"
        block
        @click="answer('allow-10-min')"
      />
    </template>
  </ConfirmDialog>
</template>
