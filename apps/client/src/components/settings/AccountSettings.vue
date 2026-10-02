<script setup lang="ts">
import type { StatusInfo } from '@renderer/components/settings/StatusLine.vue'
import StatusLine from '@renderer/components/settings/StatusLine.vue'
import FormSection from '@renderer/components/ui/FormSection.vue'
import { useAreYouSureModal } from '@renderer/composables/useAreYouSureModal'
import { useAuth } from '@renderer/composables/useAuth'
import { useWebsocketAuth } from '@renderer/composables/useWebsocketAuth'
import { hostStatus, protocolMismatchReason, serverProtocolVersion, unsupportedControlTypes } from '@renderer/composables/useWebsocketHost'
import { isValidServerWsUrl, useWebsocketSettings } from '@renderer/composables/useWebsocketSettings'
import { api } from '@renderer/lib/tauri-bridge'
import { CONTROL_TYPE_LABELS, PROTOCOL_VERSION } from '@vrc-osc-toolkit/shared-ui'
import _ from 'lodash'
import { computed, ref, watch } from 'vue'

const { serverWsUrl, defaultServerWsUrl, isCustom, setServerWsUrl } = useWebsocketSettings()
const { knownServers, hasToken, loggedIn, user, setToken, forgetServer } = useAuth()
const { authUrl } = useWebsocketAuth()
const { openModal: confirm } = useAreYouSureModal()

// Sign out forgets this server's token only; tokens for other servers stay.
const signOut = async (): Promise<void> => {
  const ok = await confirm({
    title: 'Sign out?',
    message: 'Your share link stops working until you sign in again, and connected viewers are disconnected.',
    confirmText: 'Sign out'
  })

  if (ok) setToken(undefined)
}

const draft = ref(serverWsUrl.value)
const error = ref<string>()

// The picker under the URL field: the default server first, then every server signed in to at least
// once. wss:// is left off the label as the expected case; ws:// stays visible.
const serverChips = computed(() =>
  [...new Set([defaultServerWsUrl, ...knownServers.value])].map((url) => ({
    url,
    label: url.replace(/^wss:\/\//, '').replace(/\/$/, ''),
    isDefault: url === defaultServerWsUrl,
    signedIn: hasToken(url),
    current: url === serverWsUrl.value
  }))
)

watch(serverWsUrl, (value) => (draft.value = value))

// A blank field is ignored rather than committed, so clearing it to retype doesn't reset
// anything.
const apply = (): void => {
  const trimmed = draft.value.trim()

  if (!trimmed) {
    error.value = undefined

    return
  }

  if (!isValidServerWsUrl(trimmed)) {
    error.value = 'Enter a ws:// or wss:// address.'

    return
  }

  error.value = undefined
  setServerWsUrl(trimmed)
}

// Committed shortly after typing stops, or immediately on Enter/blur.
const debouncedApply = _.debounce(apply, 500)

watch(draft, () => debouncedApply())

const commit = (): void => {
  debouncedApply.cancel()
  apply()
}

const reset = (): void => {
  debouncedApply.cancel()
  error.value = undefined
  setServerWsUrl(undefined)
  draft.value = defaultServerWsUrl
}

const pickServer = (url: string): void => {
  debouncedApply.cancel()
  error.value = undefined
  setServerWsUrl(url)
}

const forget = async (url: string): Promise<void> => {
  const ok = await confirm({
    title: 'Forget this server?',
    message: `${url} is removed from your servers, and you're signed out of it.`,
    confirmText: 'Forget'
  })

  if (ok) forgetServer(url)
}

// What the header's Server chip used to show in a popover - the chip now opens this page.
const protocolText = computed(() => `Client v${PROTOCOL_VERSION}${serverProtocolVersion.value !== null ? ` · server v${serverProtocolVersion.value}` : ''}`)
const protocolCompatible = computed(
  () => !protocolMismatchReason.value && (serverProtocolVersion.value === null || serverProtocolVersion.value === PROTOCOL_VERSION)
)
const unsupportedLabels = computed(() => unsupportedControlTypes.value.map((type) => CONTROL_TYPE_LABELS[type]).join(', '))

const status = computed<StatusInfo>(() => {
  if (!loggedIn.value) return { icon: 'i-lucide-circle-dashed', class: 'text-muted', label: 'Not signed in' }

  switch (hostStatus.value) {
    case 'CONNECTING':
      return { icon: 'i-lucide-circle-dashed', class: 'text-muted animate-pulse', label: 'Connecting…' }
    case 'OPEN':
      return { icon: 'i-lucide-circle-check', class: 'text-success', label: 'Connected' }
    default:
      return { icon: 'i-lucide-circle-x', class: 'text-error', label: 'Disconnected' }
  }
})
</script>

<template>
  <FormSection title="Account">
    <div
      v-if="loggedIn && user"
      class="flex flex-wrap items-center gap-3.5"
    >
      <UAvatar
        :src="user.discord.avatar"
        :alt="user.discord.name"
        size="xl"
      />
      <div class="grid min-w-0 flex-1">
        <b class="truncate text-[15px] font-semibold text-highlighted">{{ user.discord.name }}</b>
        <small class="text-[13px] text-muted">Signed in with Discord</small>
      </div>
      <UButton
        icon="i-lucide-log-out"
        color="error"
        variant="soft"
        @click="signOut"
      >
        Sign out
      </UButton>
    </div>
    <template v-else>
      <p class="text-[13px] text-muted">
        Sign in so viewers can reach your controls through your share link.
      </p>
      <UButton
        icon="ic:baseline-discord"
        size="xl"
        class="w-fit bg-[#5865f2] text-white hover:bg-[#4752c4]"
        :disabled="!authUrl"
        @click="api.openUrl(authUrl!)"
      >
        Sign in with Discord
      </UButton>
    </template>
  </FormSection>

  <FormSection title="Server">
    <UFormField
      label="Server URL"
      :description="isCustom ? undefined : `Using the default (${defaultServerWsUrl})`"
      :error="error"
    >
      <UInput
        v-model="draft"
        icon="i-lucide-globe"
        size="xl"
        :placeholder="defaultServerWsUrl"
        class="w-full"
        :ui="{ base: 'h-12 font-mono text-[13px]', trailing: 'pe-1.5' }"
        @keyup.enter="commit"
        @blur="commit"
      >
        <template
          v-if="isCustom"
          #trailing
        >
          <UButton
            icon="i-lucide-rotate-ccw"
            color="neutral"
            variant="ghost"
            size="sm"
            @click="reset"
          >
            Reset
          </UButton>
        </template>
      </UInput>
    </UFormField>

    <div class="grid grid-cols-1 gap-2">
      <span class="text-sm font-medium text-default">Servers you signed in to</span>
      <div class="flex flex-wrap gap-2">
        <div
          v-for="chip in serverChips"
          :key="chip.url"
          class="flex h-10.5 max-w-full min-w-0 items-center rounded-full border transition-colors"
          :class="chip.current ? 'border-primary/60 bg-primary/15 text-highlighted' : 'border-default bg-elevated/60 text-default hover:border-accented'"
        >
          <button
            type="button"
            class="flex h-full min-w-0 cursor-pointer items-center gap-1.5 ps-3.5 text-[13.5px]"
            :class="chip.isDefault ? 'pe-3.5' : 'pe-1'"
            :aria-pressed="chip.current"
            :title="`${chip.url}${chip.signedIn ? ' · signed in' : ' · signed out'}`"
            @click="pickServer(chip.url)"
          >
            <!-- Filled while a sign-in is stored for that server, hollow once signed out. -->
            <span
              class="size-2 shrink-0 rounded-full"
              :class="chip.signedIn ? 'bg-success' : 'border border-(--ui-text-dimmed)'"
            />
            <span class="truncate">{{ chip.label }}</span>
            <small
              v-if="chip.isDefault"
              class="text-xs text-muted"
            >default</small>
          </button>
          <UButton
            v-if="!chip.isDefault"
            icon="i-lucide-x"
            color="neutral"
            variant="link"
            size="sm"
            class="me-1 rounded-full"
            :aria-label="`Forget ${chip.url}`"
            @click="forget(chip.url)"
          />
        </div>
      </div>
      <p class="text-xs text-muted">
        A server is kept here once you've signed in to it, even after signing out.
      </p>
    </div>

    <div class="flex flex-wrap items-center gap-x-3 gap-y-1">
      <StatusLine :status="status" />
      <p
        class="font-mono text-xs"
        :class="protocolCompatible ? 'text-muted' : 'text-error'"
      >
        Protocol: {{ protocolText }}
      </p>
    </div>
    <UAlert
      v-if="protocolMismatchReason"
      color="error"
      variant="subtle"
      icon="i-lucide-triangle-alert"
      title="This app needs an update to use this server"
      :description="protocolMismatchReason"
    />
    <UAlert
      v-if="unsupportedControlTypes.length"
      color="warning"
      variant="subtle"
      icon="i-lucide-triangle-alert"
      title="Some controls won't show for viewers"
      :description="`This server can't display these control types yet and should be updated: ${unsupportedLabels}`"
    />
  </FormSection>
</template>
