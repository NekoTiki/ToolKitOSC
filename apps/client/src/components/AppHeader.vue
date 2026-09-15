<script setup lang="ts">
import type { DropdownMenuItem } from '@nuxt/ui/components/DropdownMenu.vue'
import { useAuth } from '@renderer/composables/useAuth'
import { useAvatarDetails } from '@renderer/composables/useAvatarDetails'
import { useLockedControls } from '@renderer/composables/useLockedControls'
import { useLockedControlsModal } from '@renderer/composables/useLockedControlsModal'
import { useSettingsModal } from '@renderer/composables/useSettingsModal'
import { useWebsocketAuth } from '@renderer/composables/useWebsocketAuth'
import { api } from '@renderer/lib/tauri-bridge'
import { computed } from 'vue'

const { lockedControlGroups, currentLockedControlsGroup } = useLockedControls()
const { openModal, listOpen } = useLockedControlsModal()
const { openModal: openSettingsModal } = useSettingsModal()
const { avatarDetails } = useAvatarDetails()
const { authUrl } = useWebsocketAuth()
const { loggedIn } = useAuth()

const openUrl = (url: string): void => {
  api.openUrl(url)
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
  above that scrolling content but well under any modal/dropdown's own layer (those sit much
  higher, e.g. UModal's z-50). -->
  <UCard
    class="sticky top-0 z-10"
    :ui="{
      body: 'p-0 sm:p-0 grid h-full divide-y divide-accented',
      root: 'overflow-visible'
    }"
  >
    <div class="flex items-center justify-between px-4 py-3.5">
      <!-- Avatar details (name/id/hash) live in WebSocket.vue's OSC status popover now, not a
      separate header icon. -->
      <WebSocket
        v-if="loggedIn"
        class="col-span-2"
      />
      <UButton
        v-else
        :disabled="!authUrl"
        icon="ic:baseline-discord"
        @click="openUrl(authUrl!)"
      >
        Sign In
      </UButton>

      <!-- z-20: Nuxt UI overlays don't set their own z-index by default, so without this they'd
      render behind this header's own z-10 (its portaled content is a sibling of the header in the
      DOM, not a descendant, so it doesn't just inherit/exceed the header's stacking level). -->
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
          :ui="{ content: 'z-20' }"
        />

        <UDropdownMenu
          :items="items"
          :content="{
            align: 'end',
            side: 'bottom',
            sideOffset: 8
          }"
          :ui="{ content: 'z-20' }"
        >
          <UButton
            icon="i-lucide-lock"
            color="neutral"
            variant="outline"
            aria-label="Manage Locked Control Profiles"
          />
        </UDropdownMenu>

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
