<script setup lang="ts">
import type { DropdownMenuItem } from '@nuxt/ui/components/DropdownMenu.vue'
import { useAuth } from '@renderer/composables/useAuth'
import { useAvatarDetails } from '@renderer/composables/useAvatarDetails'
import { useClientsListDrawer } from '@renderer/composables/useClientsListDrawer'
import { useControls } from '@renderer/composables/useControls'
import { useLockedControls } from '@renderer/composables/useLockedControls'
import { useLockedControlsModal } from '@renderer/composables/useLockedControlsModal'
import { useSettingsModal } from '@renderer/composables/useSettingsModal'
import { useWebsocketAuth } from '@renderer/composables/useWebsocketAuth'
import { api } from '@renderer/lib/tauri-bridge'
import { computed } from 'vue'

const { lockedControlGroups, currentLockedControlsGroup } = useLockedControls()
const { openModal, listOpen } = useLockedControlsModal()
const { openModal: openSettingsModal } = useSettingsModal()
const { openDrawer: openClientsListDrawer } = useClientsListDrawer()
const { avatarDetails } = useAvatarDetails()
const { addGroup } = useControls()
const { authUrl } = useWebsocketAuth()
const { loggedIn } = useAuth()

const openUrl = (url: string): void => {
  api.openUrl(url)
}

const items = computed(() => [
  { type: 'label', label: 'Divers', icon: 'lucide:ellipsis-vertical', ui: { label: 'text-muted' } },
  {
    label: 'Client List',
    icon: 'i-lucide-users',
    onSelect: () => openClientsListDrawer()
  },
  { type: 'label', label: 'Control Group', icon: 'lucide:list-tree', ui: { label: 'text-muted' } },
  {
    label: 'Add',
    icon: 'i-lucide-plus',
    color: 'primary',
    onSelect: addGroup
  },
  { type: 'separator' },
  { type: 'label', label: 'Locked Controls', icon: 'i-lucide-lock', ui: { label: 'text-muted' } },
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

        <UButton
          icon="i-lucide-settings"
          color="neutral"
          variant="outline"
          aria-label="Settings"
          @click="openSettingsModal()"
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
            icon="i-lucide-menu"
            color="neutral"
            variant="outline"
          />
        </UDropdownMenu>
      </div>
    </div>
  </UCard>
</template>

<style scoped></style>
