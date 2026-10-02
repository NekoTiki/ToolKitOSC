<script setup lang="ts">
import ChoiceCard from '@renderer/components/ui/ChoiceCard.vue'
import FormSection from '@renderer/components/ui/FormSection.vue'
import { useAuth } from '@renderer/composables/useAuth'
import type { ClientType } from '@renderer/composables/useClientType'
import { useClientType } from '@renderer/composables/useClientType'
import { serverHttpUrl } from '@renderer/composables/useWebsocketSettings'
import { api } from '@renderer/lib/tauri-bridge'
import { encodeShareCode } from '@vrc-osc-toolkit/shared-ui'
import { computed } from 'vue'

const { user, loggedIn } = useAuth()
const { clientType, setClient } = useClientType()
const toast = useToast()

// Keyed by the host's Discord id (see the server's host.ts), as a reversible short code.
const shareUrl = computed(() => (user.value?.discord?.id ? `${serverHttpUrl.value}/s/${encodeShareCode(user.value.discord.id)}` : null))

const copy = async (): Promise<void> => {
  if (!shareUrl.value) return

  try {
    await navigator.clipboard.writeText(shareUrl.value)
    toast.add({ title: 'Share link copied', icon: 'i-lucide-check', color: 'success' })
  } catch {
    toast.add({ title: 'Could not copy the share link', color: 'error' })
  }
}

const OPTIONS: { value: ClientType; label: string; description: string; icon: string }[] = [
  { value: 'everyone', label: 'Everyone', description: 'Anyone with the link, no sign-in.', icon: 'i-lucide-lock-open' },
  { value: 'username', label: 'Ask for a name', description: 'Viewers type a display name first.', icon: 'i-lucide-user-round-pen' },
  { value: 'discord', label: 'Discord only', description: 'Viewers must sign in with Discord. Best for bans.', icon: 'ic:baseline-discord' }
]
</script>

<template>
  <FormSection
    title="Your share link"
    description="Anyone with this link sees the groups you haven't hidden. Hidden groups never leave this app."
  >
    <div
      v-if="loggedIn && shareUrl"
      class="flex min-h-13.5 items-center gap-2 rounded-field well py-1.5 pr-1.5 pl-3.5 font-mono text-[13px]"
    >
      <span class="min-w-0 flex-1 truncate">{{ shareUrl }}</span>
      <UButton
        icon="i-lucide-copy"
        size="md"
        color="neutral"
        variant="subtle"
        @click="copy"
      >
        Copy
      </UButton>
      <UButton
        icon="i-lucide-external-link"
        size="md"
        color="neutral"
        variant="ghost"
        @click="api.openUrl(shareUrl)"
      >
        Open
      </UButton>
    </div>
    <UAlert
      v-else
      color="warning"
      variant="subtle"
      icon="i-lucide-triangle-alert"
      title="Sign in to get a share link"
      description="The link is tied to your Discord account. Sign in under Account & server."
    />
  </FormSection>

  <FormSection
    title="Who can use your controls"
    description="Applies to every viewer of your share link."
  >
    <div class="grid gap-2.5 sm:grid-cols-3">
      <ChoiceCard
        v-for="option in OPTIONS"
        :key="option.value"
        :title="option.label"
        :description="option.description"
        :icon="option.icon"
        :selected="clientType === option.value"
        @click="setClient(option.value)"
      />
    </div>
  </FormSection>
</template>
