<script setup lang="ts">
import ChoiceCard from '@renderer/components/ui/ChoiceCard.vue'
import FormSection from '@renderer/components/ui/FormSection.vue'
import { useAreYouSureModal } from '@renderer/composables/useAreYouSureModal'
import { useAuth } from '@renderer/composables/useAuth'
import type { ClientType } from '@renderer/composables/useClientType'
import { useClientType } from '@renderer/composables/useClientType'
import { useShareLink } from '@renderer/composables/useShareLink'
import { useShareSettings } from '@renderer/composables/useShareSettings'
import { serverHttpUrl } from '@renderer/composables/useWebsocketSettings'
import { api } from '@renderer/lib/tauri-bridge'
import { isValidShareName } from '@toolkitosc/shared-ui'
import { ref, watch } from 'vue'

const { loggedIn } = useAuth()
const { clientType, setClient } = useClientType()
const { showViewers, setShowViewers } = useShareSettings()
const toast = useToast()

const { links, shareUrl, reset, setName, setOfflineMessage } = useShareLink()
const { openModal: confirm } = useAreYouSureModal()

const nameDraft = ref('')
const nameError = ref<string | null>(null)

watch(() => links.value?.custom, (custom) => (nameDraft.value = custom ?? ''), { immediate: true })

const awayDraft = ref('')

watch(() => links.value?.offlineMessage, (message) => (awayDraft.value = message ?? ''), { immediate: true })

const saveAway = async (): Promise<void> => {
  const message = awayDraft.value.trim() || null

  if (message === (links.value?.offlineMessage ?? null)) return

  try {
    await setOfflineMessage(message)
    toast.add({ title: message ? 'Away message saved' : 'Away message cleared', icon: 'i-lucide-check', color: 'success' })
  } catch (error) {
    toast.add({ title: 'Could not save the away message', description: (error as Error).message, color: 'error' })
  }
}

const saveName = async (): Promise<void> => {
  const name = nameDraft.value.trim().toLowerCase() || null

  nameError.value = null

  if (name === (links.value?.custom ?? null)) return
  if (name && !isValidShareName(name)) {
    nameError.value = 'Use 3 to 24 lowercase letters, digits or dashes'
    return
  }

  try {
    await setName(name)
  } catch (error) {
    nameError.value = (error as Error).message
  }
}

const resetLink = async (): Promise<void> => {
  const ok = await confirm({
    title: 'Reset your share link?',
    message: 'You get a new random link. Your current one, and links from older versions of the app, **stop working**. Your custom name keeps working.',
    confirmText: 'Reset link'
  })

  if (!ok) return

  try {
    await reset()
    toast.add({ title: 'New share link ready', icon: 'i-lucide-check', color: 'success' })
  } catch (error) {
    toast.add({ title: 'Could not reset the share link', description: (error as Error).message, color: 'error' })
  }
}

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
  { value: 'discord', label: 'Discord only', description: 'Viewers must sign in with Discord. Best for bans.', icon: 'ic:baseline-discord' },
  { value: 'approved', label: 'Only people you let in', description: 'Viewers sign in with Discord, then wait until you let them in from Viewers.', icon: 'i-lucide-door-open' }
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

    <template v-if="loggedIn && links">
      <UFormField
        label="Custom name"
        :description="`Optional. Your link becomes ${serverHttpUrl}/s/<name>.`"
        :error="nameError ?? undefined"
      >
        <UInput
          v-model="nameDraft"
          icon="i-lucide-at-sign"
          size="xl"
          placeholder="your-name"
          class="w-full"
          :ui="{ base: 'h-12 font-mono text-[13px]' }"
          @keyup.enter="saveName"
          @blur="saveName"
        />
      </UFormField>

      <UFormField
        v-if="links.offlineMessage !== undefined"
        label="Away message"
        description="Shown on your share page while the app is closed, with when you were last live."
      >
        <UInput
          v-model="awayDraft"
          icon="i-lucide-moon"
          size="xl"
          placeholder="Back Friday at 9 pm"
          class="w-full"
          :maxlength="200"
          :ui="{ base: 'h-12' }"
          @keyup.enter="saveAway"
          @blur="saveAway"
        />
      </UFormField>

      <div class="flex flex-wrap items-center justify-between gap-3">
        <p class="text-[13px] text-muted">
          Link shared somewhere it shouldn't be? Reset it to get a new one.
        </p>
        <UButton
          icon="i-lucide-rotate-ccw"
          color="error"
          variant="soft"
          @click="resetLink"
        >
          Reset link
        </UButton>
      </div>
    </template>
  </FormSection>

  <FormSection
    title="Who can use your controls"
    description="Applies to every viewer of your share link."
  >
    <div class="grid gap-2.5 sm:grid-cols-2 xl:grid-cols-4">
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

  <FormSection>
    <USwitch
      :model-value="showViewers"
      label="Show viewers to each other"
      description="Viewers see who else is on your share page, and who pressed what. Turn it off to keep names private; actions still show, without a name."
      @update:model-value="setShowViewers(!!$event)"
    />
  </FormSection>
</template>
