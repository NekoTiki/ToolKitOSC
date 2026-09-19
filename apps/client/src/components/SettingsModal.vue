<script setup lang="ts">
import { useAppUpdater } from '@renderer/composables/useAppUpdater'
import { useAuth } from '@renderer/composables/useAuth'
import type { ClientType } from '@renderer/composables/useClientType'
import { useClientType } from '@renderer/composables/useClientType'
import { useIntiface } from '@renderer/composables/useIntiface'
import { useOpenShock } from '@renderer/composables/useOpenShock'
import { useSteamVrLaunch } from '@renderer/composables/useSteamVrLaunch'
import { COLOR_FAMILIES, useTheme } from '@renderer/composables/useTheme'
import { useTraySettings } from '@renderer/composables/useTraySettings'
import { useVrcxLaunch } from '@renderer/composables/useVrcxLaunch'
import { hostStatus } from '@renderer/composables/useWebsocketHost'
import { isValidServerWsUrl, useWebsocketSettings } from '@renderer/composables/useWebsocketSettings'
import { getVersion } from '@tauri-apps/api/app'
import _ from 'lodash'
import { computed, onMounted, ref, watch } from 'vue'

const open = defineModel<boolean>('open')
const { selectedPrimary, setTheme } = useTheme()
const { clientType, setClient } = useClientType()
const { serverWsUrl, defaultServerWsUrl, isCustom, setServerWsUrl } = useWebsocketSettings()
const { knownServers, loggedIn } = useAuth()
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
const { enabled: minimizeToTrayEnabled, setEnabled: setMinimizeToTrayEnabled } = useTraySettings()
const {
  enabled: steamVrEnabled,
  setEnabled: setSteamVrEnabled,
  status: steamVrStatus,
  error: steamVrError,
  available: steamVrAvailable
} = useSteamVrLaunch()
const {
  enabled: vrcxEnabled,
  setEnabled: setVrcxEnabled,
  status: vrcxStatus,
  error: vrcxError,
  available: vrcxAvailable
} = useVrcxLaunch()
const { checking: checkingForUpdates, checkForUpdates } = useAppUpdater()

const appVersion = ref<string>()

onMounted(() => void getVersion().then((version) => (appVersion.value = version)))

// Editable draft of the WS URL: mirrors the effective URL. In autocomplete mode UInputMenu fires
// `change` on every keystroke (not just blur/enter like a plain UInput), so committing straight
// off that event used to reconnect - and briefly fall back to the default server - on every
// keystroke, including the moment the field was emptied mid-edit.
const wsUrlDraft = ref(serverWsUrl.value)
const wsUrlError = ref<string | undefined>()

// Servers previously logged into, offered as suggestions when picking a server - the default is
// always reachable via the reset button, so it's left out unless the user has actually signed
// into it before.
const serverSuggestions = computed(() => knownServers.value.map((url) => ({ label: url })))

watch(serverWsUrl, (value) => (wsUrlDraft.value = value))

// Validates and, if valid, commits the draft. A blank draft (still typing, or the user cleared
// the field) is left alone rather than falling back to the default - only an explicit reset
// (resetWsUrl) or a valid URL changes the active server.
const applyWsUrl = (): void => {
  const trimmed = wsUrlDraft.value.trim()

  if (!trimmed) {
    wsUrlError.value = undefined
    return
  }

  if (!isValidServerWsUrl(trimmed)) {
    wsUrlError.value = 'Enter a valid ws:// or wss:// URL'
    return
  }

  wsUrlError.value = undefined
  setServerWsUrl(trimmed)
}

const WS_URL_COMMIT_DEBOUNCE_MS = 500

// Debounced so a valid URL is committed (and reconnected to) shortly after the user stops typing,
// without reconnecting on every keystroke.
const debouncedApplyWsUrl = _.debounce(applyWsUrl, WS_URL_COMMIT_DEBOUNCE_MS)

watch(wsUrlDraft, () => debouncedApplyWsUrl())

// Enter/blur commit (or report an error) immediately instead of waiting out the debounce.
const commitWsUrl = (): void => {
  debouncedApplyWsUrl.cancel()
  applyWsUrl()
}

const resetWsUrl = (): void => {
  debouncedApplyWsUrl.cancel()
  wsUrlError.value = undefined
  setServerWsUrl(undefined)
  wsUrlDraft.value = defaultServerWsUrl
}

// The host connection only opens once the user is signed in (see AppHeader.vue/StatusBar.vue), so
// that's checked first - otherwise hostStatus would just be stuck reading its initial 'CLOSED'
// forever and misreport as a failed connection rather than "not signed in yet".
const serverStatusInfo = computed(() => {
  if (!loggedIn.value) {
    return { icon: 'i-lucide-circle-dashed', class: 'text-muted', label: 'Not signed in' }
  }

  switch (hostStatus.value) {
    case 'CONNECTING':
      // Same reasoning as openShockStatusInfo's 'checking' case below - animate-pulse, not spin.
      return { icon: 'i-lucide-circle-dashed', class: 'text-muted animate-pulse', label: 'Connecting…' }
    case 'OPEN':
      return { icon: 'i-lucide-check-circle', class: 'text-success', label: 'Connected' }
    default:
      return { icon: 'i-lucide-circle-x', class: 'text-error', label: 'Disconnected' }
  }
})

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
      // A spinning icon (animate-spin) swapping out for a static one mid-rotation looked like a
      // glitch - it could land at any angle when the icon changed. animate-pulse is a plain
      // opacity fade, so there's no rotation to freeze awkwardly when the status resolves.
      return { icon: 'i-lucide-circle-dashed', class: 'text-muted animate-pulse', label: 'Checking key…' }
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
      // Same reasoning as openShockStatusInfo's 'checking' case above - animate-pulse, not spin.
      return { icon: 'i-lucide-circle-dashed', class: 'text-muted animate-pulse', label: 'Connecting…' }
    case 'connected':
      return { icon: 'i-lucide-check-circle', class: 'text-success', label: 'Connected' }
    case 'error':
      return { icon: 'i-lucide-circle-x', class: 'text-error', label: 'Connection failed' }
    default:
      return { icon: 'i-lucide-circle-dashed', class: 'text-muted', label: 'Disabled' }
  }
})

const steamVrStatusInfo = computed(() => {
  switch (steamVrStatus.value) {
    case 'checking':
      return { icon: 'i-lucide-circle-dashed', class: 'text-muted animate-pulse', label: 'Registering with SteamVR…' }
    case 'enabled':
      return { icon: 'i-lucide-check-circle', class: 'text-success', label: 'Enabled' }
    case 'error':
      return { icon: 'i-lucide-circle-x', class: 'text-error', label: steamVrError.value || 'Failed' }
    default:
      return { icon: 'i-lucide-circle-dashed', class: 'text-muted', label: 'Disabled' }
  }
})

const vrcxStatusInfo = computed(() => {
  switch (vrcxStatus.value) {
    case 'enabled':
      return { icon: 'i-lucide-check-circle', class: 'text-success', label: 'Enabled' }
    case 'error':
      return { icon: 'i-lucide-circle-x', class: 'text-error', label: vrcxError.value || 'Failed' }
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
  { label: 'Connections', icon: 'i-lucide-plug', slot: 'connections' },
  { label: 'Launch', icon: 'i-lucide-rocket', slot: 'launch' }
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
    :ui="{ header: 'p-4 sm:p-4 text-lg', body: 'p-4 sm:p-4' }"
  >
    <template #body>
      <UTabs
        :items="settingsTabs"
        :ui="{ trigger: 'flex-1 basis-0', content: 'flex flex-col gap-6 pt-4' }"
      >
        <template #connections>
          <div class="flex flex-col gap-2">
            <h3 class="text-xs font-semibold text-highlighted">
              Connection
            </h3>
            <UFormField
              label="Server URL"
              :description="isCustom ? undefined : `Using the default (${defaultServerWsUrl})`"
              :error="wsUrlError"
            >
              <div class="flex gap-2">
                <UInputMenu
                  v-model="wsUrlDraft"
                  mode="autocomplete"
                  :items="serverSuggestions"
                  value-key="label"
                  ignore-filter
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
              <div class="mt-1.5 flex items-center gap-1.5 text-xs">
                <UIcon
                  :name="serverStatusInfo.icon"
                  class="size-3.5"
                  :class="serverStatusInfo.class"
                />
                <span :class="serverStatusInfo.class">{{ serverStatusInfo.label }}</span>
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

          <div class="flex flex-col gap-2">
            <h3 class="text-xs font-semibold text-highlighted">
              Updates
            </h3>
            <div class="flex items-center justify-between gap-2 rounded-lg border border-default p-3">
              <div>
                <p class="text-sm font-medium">
                  VRC OSC Toolkit
                </p>
                <p class="text-xs text-muted">
                  {{ appVersion ? `Version ${appVersion}` : 'Checking version…' }}
                </p>
              </div>
              <UButton
                icon="i-lucide-refresh-cw"
                color="neutral"
                variant="outline"
                :loading="checkingForUpdates"
                @click="checkForUpdates()"
              >
                Check for Updates
              </UButton>
            </div>
          </div>
        </template>

        <template #launch>
          <div class="flex flex-col gap-2">
            <h3 class="text-xs font-semibold text-highlighted">
              Launch Options
            </h3>
            <UFormField
              label="Keep running in the tray"
              description="Closing the window keeps the app running in the system tray instead of quitting it."
            >
              <USwitch
                :model-value="minimizeToTrayEnabled"
                @update:model-value="setMinimizeToTrayEnabled(!!$event)"
              />
            </UFormField>
            <UFormField
              v-if="steamVrAvailable"
              label="Launch with SteamVR"
              description="Registers this app with SteamVR so it starts automatically whenever SteamVR does."
            >
              <USwitch
                :model-value="steamVrEnabled"
                @update:model-value="setSteamVrEnabled(!!$event)"
              />
              <div
                v-if="steamVrEnabled"
                class="mt-1.5 flex items-center gap-1.5 text-xs"
              >
                <UIcon
                  :name="steamVrStatusInfo.icon"
                  class="size-3.5"
                  :class="steamVrStatusInfo.class"
                />
                <span :class="steamVrStatusInfo.class">{{ steamVrStatusInfo.label }}</span>
              </div>
            </UFormField>
            <UFormField
              v-if="vrcxAvailable"
              label="Launch with VRChat"
              description="Adds this app to VRCX's Auto-Launch Folder, so it starts whenever VRChat does."
            >
              <USwitch
                :model-value="vrcxEnabled"
                @update:model-value="setVrcxEnabled(!!$event)"
              />
              <div
                v-if="vrcxEnabled"
                class="mt-1.5 flex items-center gap-1.5 text-xs"
              >
                <UIcon
                  :name="vrcxStatusInfo.icon"
                  class="size-3.5"
                  :class="vrcxStatusInfo.class"
                />
                <span :class="vrcxStatusInfo.class">{{ vrcxStatusInfo.label }}</span>
              </div>
            </UFormField>
          </div>
        </template>
      </UTabs>
    </template>
  </UModal>
</template>

<style scoped></style>
