<script setup lang="ts">
import type { DropdownMenuItem } from '@nuxt/ui/components/DropdownMenu.vue'
import { useAuth } from '@renderer/composables/useAuth'
import { useAvatarDetails } from '@renderer/composables/useAvatarDetails'
import { useLockedControls } from '@renderer/composables/useLockedControls'
import { useLockedControlsModal } from '@renderer/composables/useLockedControlsModal'
import { usePresetModal } from '@renderer/composables/usePresetModal'
import { useSettingsModal } from '@renderer/composables/useSettingsModal'
import { useWebsocketAuth } from '@renderer/composables/useWebsocketAuth'
import { serverHttpUrl } from '@renderer/composables/useWebsocketSettings'
import { api } from '@renderer/lib/tauri-bridge'
import { computed, useTemplateRef } from 'vue'

const { lockedControlGroups, currentLockedControlsGroup } = useLockedControls()
const { openModal, listOpen } = useLockedControlsModal()
const { listOpen: presetListOpen } = usePresetModal()
const { openModal: openSettingsModal } = useSettingsModal()
const { avatarDetails } = useAvatarDetails()
const { authUrl } = useWebsocketAuth()
const { user, loggedIn } = useAuth()
const toast = useToast()

// Anchor for every status popover in StatusBar.vue - passed down so they all open flush with the
// header bar's own left edge, right below it, rather than each centering under its own icon.
const headerBarEl = useTemplateRef('headerBarEl')

const openUrl = (url: string): void => {
  api.openUrl(url)
}

// The share page's room is keyed by the host's Discord id (see the server's host.ts
// `getRoomId`), so there's nothing to share until that's known.
const shareUrl = computed<string | null>(() =>
  user.value?.discord?.id ? `${serverHttpUrl.value}/share/${user.value.discord.id}` : null
)

const copyShareLink = async (): Promise<void> => {
  if (!shareUrl.value) return

  try {
    await navigator.clipboard.writeText(shareUrl.value)
    toast.add({ title: 'Share link copied', icon: 'i-lucide-check', color: 'success' })
  } catch {
    toast.add({ title: 'Could not copy share link', color: 'error' })
  }
}

const items = computed(() => [
  {
    label: 'Add',
    icon: 'i-lucide-plus',
    color: 'primary',
    onSelect: () => openModal()
  },
  {
    label: 'Edit',
    icon: 'i-lucide-pen',
    onSelect: () => (listOpen.value = true)
  }
  // Cast needed because @nuxt/ui's generated `ui` slot type (Pick<DropdownMenu['slots'], ...>)
  // requires every picked slot key, not just the ones we actually set (e.g. `label`).
] as DropdownMenuItem[])
</script>

<template>
  <!-- sticky: stays in view while the control groups below scroll underneath it. z-10 keeps it
  above that scrolling content - every overlay that can render inside it (or anywhere else in the
  app) is set well above this globally in vite.config.ts, so nothing here needs its own override. -->
  <UCard
    class="sticky top-0 z-10"
    :ui="{
      body: 'p-0 sm:p-0 grid h-full divide-y divide-accented',
      root: 'overflow-visible'
    }"
  >
    <div
      ref="headerBarEl"
      class="flex items-center justify-between px-4 py-3.5"
    >
      <!-- Avatar details (name/id/hash) live in StatusBar.vue's OSC status popover now, not a
      separate header icon. -->
      <StatusBar
        v-if="loggedIn"
        class="col-span-2"
        :reference="headerBarEl"
      />
      <UButton
        v-else
        :disabled="!authUrl"
        icon="ic:baseline-discord"
        @click="openUrl(authUrl!)"
      >
        Sign In
      </UButton>

      <div class="flex items-center gap-2">
        <USelect
          v-model="currentLockedControlsGroup"
          :items="[
            { type: 'label', label: 'Locked Controls Profile' },
            { label: 'None', value: null },
            ...lockedControlGroups.map((g) => ({ label: g.name, value: g.id }))
          ]"
          :disabled="!avatarDetails?.id"
          icon="i-lucide-lock"
          class="min-w-52"
        />

        <UDropdownMenu
          :items="items"
          :content="{
            align: 'end',
            side: 'bottom',
            sideOffset: 8
          }"
        >
          <UButton
            icon="i-lucide-lock"
            color="neutral"
            variant="outline"
            aria-label="Manage Locked Control Profiles"
          />
        </UDropdownMenu>

        <UTooltip>
          <UButton
            icon="i-lucide-layers"
            color="neutral"
            variant="outline"
            :disabled="!avatarDetails?.id"
            aria-label="Manage Presets"
            @click="presetListOpen = true"
          />

          <template #content>
            Manage Avatar Presets
          </template>
        </UTooltip>

        <UTooltip v-if="loggedIn">
          <UButton
            icon="i-lucide-share-2"
            color="neutral"
            variant="outline"
            :disabled="!shareUrl"
            aria-label="Copy Share Link"
            @click="copyShareLink"
          />

          <template #content>
            Copy the share link to your control page
          </template>
        </UTooltip>

        <UButton
          icon="i-lucide-settings"
          color="neutral"
          variant="outline"
          aria-label="Settings"
          @click="openSettingsModal()"
        />
      </div>
    </div>
  </UCard>
</template>

<style scoped></style>
