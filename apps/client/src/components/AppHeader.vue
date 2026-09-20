<script setup lang="ts">
import type { DropdownMenuItem } from '@nuxt/ui/components/DropdownMenu.vue'
import { useAiAccess } from '@renderer/composables/useAiAccess'
import { useAiControlsModal } from '@renderer/composables/useAiControlsModal'
import { useAuth } from '@renderer/composables/useAuth'
import { useAvatarDetails } from '@renderer/composables/useAvatarDetails'
import { useControls } from '@renderer/composables/useControls'
import { useLockedControlGroupListModal } from '@renderer/composables/useLockedControlGroupListModal'
import { useLockedControlGroupModal } from '@renderer/composables/useLockedControlGroupModal'
import { useLockedControls } from '@renderer/composables/useLockedControls'
import { useManageGroupsModal } from '@renderer/composables/useManageGroupsModal'
import { useParametersModal } from '@renderer/composables/useParametersModal'
import { usePresetListModal } from '@renderer/composables/usePresetListModal'
import { useSettingsModal } from '@renderer/composables/useSettingsModal'
import { useWebsocketAuth } from '@renderer/composables/useWebsocketAuth'
import { serverHttpUrl } from '@renderer/composables/useWebsocketSettings'
import { api } from '@renderer/lib/tauri-bridge'
import { encodeShareCode } from '@vrc-osc-toolkit/shared-ui'
import { computed, useTemplateRef } from 'vue'

const { lockedControlGroups, currentLockedControlsGroup } = useLockedControls()
const { openModal } = useLockedControlGroupModal()
const { openList: openLockedControlGroupList } = useLockedControlGroupListModal()
const { openList: openPresetList } = usePresetListModal()
const { openModal: openParametersModal } = useParametersModal()
const { openModal: openSettingsModal } = useSettingsModal()
const { openModal: openAiControlsModal } = useAiControlsModal()
const { openModal: openManageGroupsModal } = useManageGroupsModal()
const { avatarDetails } = useAvatarDetails()
const { authUrl } = useWebsocketAuth()
const { user, loggedIn } = useAuth()
const { access: aiAccess } = useAiAccess()
const { addGroup } = useControls()
const toast = useToast()

// Anchor for every status popover in StatusBar.vue - passed down so they all open flush with the
// header bar's own left edge, right below it, rather than each centering under its own icon.
const headerBarEl = useTemplateRef('headerBarEl')

const openUrl = (url: string): void => {
  api.openUrl(url)
}

// The share page's room is keyed by the host's Discord id (see the server's host.ts
// `getRoomId`), so there's nothing to share until that's known. The link itself uses the short
// `/s/<code>` form (see server/routes/s/[code].get.ts) - `code` is a reversible re-encoding of that
// same id, not a stored/random one, so it needs no round trip to the server to produce.
const shareUrl = computed<string | null>(() =>
  user.value?.discord?.id ? `${serverHttpUrl.value}/s/${encodeShareCode(user.value.discord.id)}` : null
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
    onSelect: () => openLockedControlGroupList()
  }
  // Cast needed because @nuxt/ui's generated `ui` slot type (Pick<DropdownMenu['slots'], ...>)
  // requires every picked slot key, not just the ones we actually set (e.g. `label`).
] as DropdownMenuItem[])

const groupMenuItems = computed(
  () =>
    [
      {
        label: 'Add Group',
        icon: 'i-lucide-plus',
        color: 'primary',
        onSelect: () => addGroup()
      },
      {
        label: 'Manage Groups',
        icon: 'i-lucide-list-checks',
        onSelect: () => openManageGroupsModal()
      }
    ] as DropdownMenuItem[]
)
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
            @click="openPresetList"
          />

          <template #content>
            Manage Avatar Presets
          </template>
        </UTooltip>

        <UTooltip>
          <UButton
            icon="i-lucide-list-tree"
            color="neutral"
            variant="outline"
            :disabled="!avatarDetails?.id"
            aria-label="View Avatar Parameters"
            @click="openParametersModal"
          />

          <template #content>
            View Avatar Parameters
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

        <UDropdownMenu
          :items="groupMenuItems"
          :content="{
            align: 'end',
            side: 'bottom',
            sideOffset: 8
          }"
        >
          <UButton
            icon="i-lucide-layout-grid"
            color="neutral"
            variant="outline"
            :disabled="!avatarDetails?.id"
            aria-label="Manage Control Groups"
          />
        </UDropdownMenu>

        <UTooltip v-if="aiAccess.hasAccess">
          <button
            type="button"
            class="ai-generate-btn"
            aria-label="AI: Suggest Controls"
            :disabled="!avatarDetails?.id"
            @click="openAiControlsModal"
          >
            <UIcon
              name="i-lucide-sparkles"
              class="size-4"
            />
          </button>

          <template #content>
            AI: Suggest Controls
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

<style scoped>
/* A themed (not hardcoded-neon) gradient so it fits whatever primary/secondary color the user has
   picked in Settings (see useTheme.ts) - the same --ui-color-* vars main.css already reads for the
   scrollbar. Sits alongside UButton rather than using its `color`/`variant` props since neither
   supports an animated multi-stop gradient background. Sized/shaped to match the plain icon
   UButtons around it (a square icon button, not the wider text+icon pill this used to be down in
   App.vue's own button row). */
.ai-generate-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  padding: 0.5rem;
  border-radius: var(--ui-radius);
  color: white;
  cursor: pointer;
  background-image: linear-gradient(
    115deg,
    var(--ui-color-primary-500),
    var(--ui-color-secondary-500),
    var(--ui-color-primary-500)
  );
  background-size: 200% 100%;
  background-position: 0% 0%;
  transition:
    box-shadow 0.15s ease,
    background-position 0.6s ease,
    opacity 0.15s ease;
}

.ai-generate-btn:hover:not(:disabled) {
  background-position: 100% 0%;
  box-shadow: 0 4px 14px color-mix(in oklab, var(--ui-color-primary-500) 45%, transparent);
}

.ai-generate-btn:disabled {
  opacity: 0.6;
  cursor: default;
}
</style>
