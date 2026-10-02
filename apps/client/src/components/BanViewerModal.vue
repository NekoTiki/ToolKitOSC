<script setup lang="ts">
import ChoiceCard from '@renderer/components/ui/ChoiceCard.vue'
import ConfirmDialog from '@renderer/components/ui/ConfirmDialog.vue'
import type { Client } from '@renderer/db/clients.db'
import type { BanScope } from '@renderer/db/clients.db'
import { ref } from 'vue'

// Confirms a ban and collects how (by IP or Discord account) and why. The reason is shown to the
// viewer on their banned screen. Stays a dialog rather than a page: it's a confirmation step.
const props = defineProps<{ client: Pick<Client, 'displayName' | 'ip' | 'discordId'> }>()
const emit = defineEmits<{ close: [result: { scope: BanScope; reason?: string } | false] }>()
const open = defineModel<boolean>('open')

const scope = ref<BanScope>(props.client.discordId ? 'discord' : 'ip')
const reason = ref('')

// The first close wins: confirming resolves with the choice, and the after:leave fallback below
// (backdrop, Escape, Cancel) then has no effect.
const confirm = (): void => {
  emit('close', { scope: scope.value, reason: reason.value.trim() || undefined })
  open.value = false
}
</script>

<template>
  <ConfirmDialog
    v-model:open="open"
    :title="`Ban ${client.displayName}?`"
    confirm-text="Ban"
    confirm-icon="i-lucide-ban"
    @confirm="confirm"
    @cancel="open = false"
    @after:leave="emit('close', false)"
  >
    <p>Banned viewers can't use any control. You can unban them later from Viewers or Settings.</p>
    <template #fields>
      <div class="grid grid-cols-2 gap-2.5">
        <ChoiceCard
          title="By IP address"
          description="Blocks anyone on this network."
          icon="i-lucide-globe"
          :selected="scope === 'ip'"
          @click="scope = 'ip'"
        />
        <ChoiceCard
          title="By Discord"
          description="Follows them to any network."
          icon="ic:baseline-discord"
          :selected="scope === 'discord'"
          :disabled="!client.discordId"
          disabled-reason="They didn't sign in with Discord"
          @click="scope = 'discord'"
        />
      </div>
      <UFormField
        label="Reason"
        description="Optional. Shown to them."
      >
        <UInput
          v-model="reason"
          placeholder="Spamming the shocker…"
          class="w-full"
          @keydown.enter="confirm"
        />
      </UFormField>
    </template>
  </ConfirmDialog>
</template>
