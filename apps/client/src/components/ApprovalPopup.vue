<script setup lang="ts">
import ConfirmDialog from '@renderer/components/ui/ConfirmDialog.vue'
import { useControls } from '@renderer/composables/useControls'
import { useMcpApprovals } from '@renderer/composables/useMcpBridge'
import { computed } from 'vue'

// An AI assistant wants to use a tool that's on Ask in Settings > Assistants. Shown over whatever
// page you're on, one at a time. Left alone for a minute, Rust counts it as Deny.
const { approvals, answerApproval } = useMcpApprovals()
const { getControl } = useControls()

const current = computed(() => approvals.value[0] ?? null)
const open = computed(() => !!current.value)

const details = computed(() => (current.value?.arguments ? JSON.stringify(current.value.arguments, null, 2) : null))

// A change to one of your controls shows the control as it is now, next to the change.
const before = computed(() => {
  const args = current.value?.arguments
  if (current.value?.tool !== 'update_control' || !args) return null
  const control = getControl(String(args.groupId), String(args.controlId))

  return control ? JSON.stringify(control, null, 2) : null
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
    :danger="false"
    @confirm="answer('allow')"
    @cancel="answer('deny')"
    @update:open="(value) => !value && answer('deny')"
  >
    <p>
      It wants to use <strong>{{ current?.tool }}</strong>.
    </p>
    <template #fields>
      <template v-if="before">
        <b class="text-sm text-highlighted">Now</b>
        <pre class="max-h-60 overflow-auto rounded-2xl bg-(--aurora-well) p-3 font-mono text-xs whitespace-pre-wrap text-default">{{ before }}</pre>
        <b class="text-sm text-highlighted">Change</b>
      </template>
      <pre
        v-if="details"
        class="max-h-60 overflow-auto rounded-2xl bg-(--aurora-well) p-3 font-mono text-xs whitespace-pre-wrap text-default"
      >{{ details }}</pre>
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
