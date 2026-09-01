<script setup lang="ts">
import type { ClientType } from '@renderer/composables/useClientType'
import { useClientType } from '@renderer/composables/useClientType'
import { useSettingsModal } from '@renderer/composables/useSettingsModal'
import { COLOR_FAMILIES, useTheme } from '@renderer/composables/useTheme'
import { useWebsocketSettings } from '@renderer/composables/useWebsocketSettings'
import { ref, watch } from 'vue'

const { open } = useSettingsModal()
const { selectedPrimary, setTheme } = useTheme()
const { clientType, setClient } = useClientType()
const { serverWsUrl, defaultServerWsUrl, isCustom, setServerWsUrl } = useWebsocketSettings()

// Editable draft of the WS URL: mirrors the effective URL, but only commits (and reconnects) on
// blur/enter rather than on every keystroke.
const wsUrlDraft = ref(serverWsUrl.value)

watch(serverWsUrl, (value) => (wsUrlDraft.value = value))

const commitWsUrl = (): void => setServerWsUrl(wsUrlDraft.value)

const resetWsUrl = (): void => {
  setServerWsUrl(undefined)
  wsUrlDraft.value = defaultServerWsUrl
}

const clientTypeOptions: { value: ClientType; label: string; description: string; icon: string }[] = [
  {
    value: 'everyone',
    label: 'Everyone',
    description: 'Allows anyone to use the controls',
    icon: 'lucide:lock-open'
  },
  {
    value: 'username',
    label: 'Ask Username',
    description: 'Ask for the username before using the controls',
    icon: 'lucide:user-round-pen'
  },
  {
    value: 'discord',
    label: 'Discord',
    description: 'Only allow users with a Discord account to use the controls',
    icon: 'ic:baseline-discord'
  }
]
</script>

<template>
  <UModal
    v-model:open="open"
    title="Settings"
    class="z-50"
    :ui="{ overlay: 'z-50', header: 'p-4 sm:p-4 text-lg', body: 'p-4 sm:p-4 flex flex-col gap-6' }"
  >
    <template #body>
      <div class="flex flex-col gap-2">
        <h3 class="text-xs font-semibold text-highlighted">
          Connection
        </h3>
        <UFormField
          label="Server URL"
          :description="isCustom ? undefined : `Using the default (${defaultServerWsUrl})`"
        >
          <div class="flex gap-2">
            <UInput
              v-model="wsUrlDraft"
              :placeholder="defaultServerWsUrl"
              class="w-full"
              @change="commitWsUrl"
              @keyup.enter="commitWsUrl"
              @blur="commitWsUrl"
            />
            <UButton
              v-if="isCustom"
              icon="i-lucide-rotate-ccw"
              color="neutral"
              variant="outline"
              aria-label="Reset to default"
              @click="resetWsUrl"
            />
          </div>
        </UFormField>
      </div>

      <div class="flex flex-col gap-2">
        <h3 class="text-xs font-semibold text-highlighted">
          Style
        </h3>
        <div class="flex flex-wrap gap-2">
          <button
            v-for="family in COLOR_FAMILIES"
            :key="family"
            type="button"
            class="relative size-8 shrink-0 rounded-full transition hover:scale-110"
            :style="{ backgroundColor: `var(--color-${family}-500)` }"
            :aria-label="family"
            :aria-pressed="selectedPrimary === family"
            @click="setTheme('primary', family)"
          >
            <UIcon
              v-if="selectedPrimary === family"
              name="i-lucide-check"
              class="absolute inset-0 m-auto text-white"
            />
          </button>
        </div>
      </div>

      <div class="flex flex-col gap-2">
        <h3 class="text-xs font-semibold text-highlighted">
          Authorized Clients
        </h3>
        <div class="flex flex-col gap-2">
          <button
            v-for="option in clientTypeOptions"
            :key="option.value"
            type="button"
            class="flex items-center gap-3 rounded-lg border border-default p-3 text-left transition hover:bg-elevated"
            :class="clientType === option.value ? 'border-primary bg-primary/10' : ''"
            @click="setClient(option.value)"
          >
            <UIcon
              :name="option.icon"
              class="size-5 shrink-0"
              :class="clientType === option.value ? 'text-primary' : 'text-muted'"
            />
            <div class="flex-1">
              <p class="text-sm font-medium">
                {{ option.label }}
              </p>
              <p class="text-xs text-muted">
                {{ option.description }}
              </p>
            </div>
            <UIcon
              v-if="clientType === option.value"
              name="i-lucide-check"
              class="text-primary"
            />
          </button>
        </div>
      </div>
    </template>
  </UModal>
</template>

<style scoped></style>
