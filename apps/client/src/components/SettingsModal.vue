<script setup lang="ts">
import type { ClientType } from '@renderer/composables/useClientType'
import { useClientType } from '@renderer/composables/useClientType'
import { useIntiface } from '@renderer/composables/useIntiface'
import { useOpenShock } from '@renderer/composables/useOpenShock'
import { useSettingsModal } from '@renderer/composables/useSettingsModal'
import { COLOR_FAMILIES, useTheme } from '@renderer/composables/useTheme'
import { useWebsocketSettings } from '@renderer/composables/useWebsocketSettings'
import { computed, ref, watch } from 'vue'

const { open } = useSettingsModal()
const { selectedPrimary, setTheme } = useTheme()
const { clientType, setClient } = useClientType()
const { serverWsUrl, defaultServerWsUrl, isCustom, setServerWsUrl } = useWebsocketSettings()
const {
  token: openShockToken,
  status: openShockStatus,
  setToken: setOpenShockToken,
  enabled: openShockEnabled,
  setEnabled: setOpenShockEnabled
} = useOpenShock()
const {
  url: intifaceUrl,
  defaultUrl: defaultIntifaceUrl,
  isCustomUrl: isCustomIntifaceUrl,
  setUrl: setIntifaceUrl,
  enabled: intifaceEnabled,
  setEnabled: setIntifaceEnabled,
  status: intifaceStatus
} = useIntiface()

// Editable draft of the WS URL: mirrors the effective URL, but only commits (and reconnects) on
// blur/enter rather than on every keystroke.
const wsUrlDraft = ref(serverWsUrl.value)

watch(serverWsUrl, (value) => (wsUrlDraft.value = value))

const commitWsUrl = (): void => setServerWsUrl(wsUrlDraft.value)

const resetWsUrl = (): void => {
  setServerWsUrl(undefined)
  wsUrlDraft.value = defaultServerWsUrl
}

// Same draft-then-commit pattern as the WS URL above: typing doesn't hit the API on every
// keystroke, only on blur/enter, which is also when setToken() kicks off validation.
const openShockTokenDraft = ref(openShockToken.value)
const openShockTokenVisible = ref(false)

watch(openShockToken, (value) => (openShockTokenDraft.value = value))

const commitOpenShockToken = (): void => {
  if (openShockTokenDraft.value !== openShockToken.value) setOpenShockToken(openShockTokenDraft.value)
}

const openShockStatusInfo = computed(() => {
  switch (openShockStatus.value) {
    case 'checking':
      return { icon: 'i-lucide-loader-2', class: 'text-muted animate-spin', label: 'Checking key…' }
    case 'valid':
      return { icon: 'i-lucide-check-circle', class: 'text-success', label: 'Connected' }
    case 'invalid':
      return { icon: 'i-lucide-circle-x', class: 'text-error', label: 'Invalid key' }
    case 'disabled':
      return { icon: 'i-lucide-circle-dashed', class: 'text-muted', label: 'Disabled' }
    default:
      return { icon: 'i-lucide-circle-dashed', class: 'text-muted', label: 'Not configured' }
  }
})

// Same draft-then-commit pattern as the WS/OpenShock fields above.
const intifaceUrlDraft = ref(intifaceUrl.value)

watch(intifaceUrl, (value) => (intifaceUrlDraft.value = value))

const commitIntifaceUrl = (): void => setIntifaceUrl(intifaceUrlDraft.value)

const resetIntifaceUrl = (): void => {
  setIntifaceUrl(undefined)
  intifaceUrlDraft.value = ''
}

const intifaceStatusInfo = computed(() => {
  switch (intifaceStatus.value) {
    case 'connecting':
      return { icon: 'i-lucide-loader-2', class: 'text-muted animate-spin', label: 'Connecting…' }
    case 'connected':
      return { icon: 'i-lucide-check-circle', class: 'text-success', label: 'Connected' }
    case 'error':
      return { icon: 'i-lucide-circle-x', class: 'text-error', label: 'Connection failed' }
    default:
      return { icon: 'i-lucide-circle-dashed', class: 'text-muted', label: 'Disabled' }
  }
})

// Config (external connections) vs. preferences (styling/who's allowed to connect) don't have much
// to do with each other and were just piling up as one long scroll - splitting them into tabs
// keeps each one short enough to see at a glance.
// Preferences listed (and selected by default - UTabs defaults to its first item) before
// Connections: most visits to Settings are to tweak style/client access, not touch a service
// connection.
const settingsTabs = [
  { label: 'Preferences', icon: 'i-lucide-sliders-horizontal', slot: 'preferences' },
  { label: 'Connections', icon: 'i-lucide-plug', slot: 'connections' }
]

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
    :ui="{ overlay: 'z-50', header: 'p-4 sm:p-4 text-lg', body: 'p-4 sm:p-4' }"
  >
    <template #body>
      <UTabs
        :items="settingsTabs"
        :ui="{ content: 'flex flex-col gap-6 pt-4' }"
      >
        <template #connections>
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
              OpenShock
            </h3>
            <UFormField
              label="Enable OpenShock"
              description="Lets OpenShock controls be created and used."
            >
              <USwitch
                :model-value="openShockEnabled"
                @update:model-value="setOpenShockEnabled(!!$event)"
              />
            </UFormField>
            <UFormField
              v-if="openShockEnabled"
              label="API Key"
              description="Generate one from your OpenShock account settings."
            >
              <div class="flex gap-2">
                <UInput
                  v-model="openShockTokenDraft"
                  :type="openShockTokenVisible ? 'text' : 'password'"
                  placeholder="Paste your OpenShock API key"
                  class="w-full"
                  autocomplete="off"
                  @change="commitOpenShockToken"
                  @keyup.enter="commitOpenShockToken"
                  @blur="commitOpenShockToken"
                >
                  <template #trailing>
                    <UButton
                      :icon="openShockTokenVisible ? 'i-lucide-eye-off' : 'i-lucide-eye'"
                      color="neutral"
                      variant="link"
                      size="xs"
                      :aria-label="openShockTokenVisible ? 'Hide API key' : 'Show API key'"
                      @click="openShockTokenVisible = !openShockTokenVisible"
                    />
                  </template>
                </UInput>
              </div>
              <div class="mt-1.5 flex items-center gap-1.5 text-xs">
                <UIcon
                  :name="openShockStatusInfo.icon"
                  class="size-3.5"
                  :class="openShockStatusInfo.class"
                />
                <span :class="openShockStatusInfo.class">{{ openShockStatusInfo.label }}</span>
              </div>
            </UFormField>
          </div>

          <div class="flex flex-col gap-2">
            <h3 class="text-xs font-semibold text-highlighted">
              Intiface
            </h3>
            <UFormField
              label="Enable Intiface"
              description="Connects to a running Intiface Central/Engine to control toys."
            >
              <USwitch
                :model-value="intifaceEnabled"
                @update:model-value="setIntifaceEnabled(!!$event)"
              />
            </UFormField>
            <UFormField
              v-if="intifaceEnabled"
              label="Server URL"
              :description="isCustomIntifaceUrl ? undefined : `Using the default (${defaultIntifaceUrl})`"
            >
              <div class="flex gap-2">
                <UInput
                  v-model="intifaceUrlDraft"
                  :placeholder="defaultIntifaceUrl"
                  class="w-full"
                  autocomplete="off"
                  @change="commitIntifaceUrl"
                  @keyup.enter="commitIntifaceUrl"
                  @blur="commitIntifaceUrl"
                />
                <UButton
                  v-if="isCustomIntifaceUrl"
                  icon="i-lucide-rotate-ccw"
                  color="neutral"
                  variant="outline"
                  aria-label="Reset to default"
                  @click="resetIntifaceUrl"
                />
              </div>
              <div class="mt-1.5 flex items-center gap-1.5 text-xs">
                <UIcon
                  :name="intifaceStatusInfo.icon"
                  class="size-3.5"
                  :class="intifaceStatusInfo.class"
                />
                <span :class="intifaceStatusInfo.class">{{ intifaceStatusInfo.label }}</span>
              </div>
            </UFormField>
          </div>
        </template>

        <template #preferences>
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
      </UTabs>
    </template>
  </UModal>
</template>

<style scoped></style>
