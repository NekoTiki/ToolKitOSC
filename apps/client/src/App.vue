<script setup lang="ts">
import AppHeader from '@renderer/components/AppHeader.vue'
import AreYouSureModal from '@renderer/components/AreYouSureModal.vue'
import ControlGroup from '@renderer/components/controls/ControlGroup.vue'
import SettingsModal from '@renderer/components/SettingsModal.vue'
import { useAvatarDetails } from '@renderer/composables/useAvatarDetails'
import { useControls } from '@renderer/composables/useControls'
import { useOscConnection } from '@renderer/composables/useOscConnection'
import { useTheme } from '@renderer/composables/useTheme'
import { api } from '@renderer/lib/tauri-bridge'
import { computed, onMounted } from 'vue'

const { controls, addGroup } = useControls()
const { loadTheme } = useTheme()
const { connected: oscConnected } = useOscConnection()
const { avatarDetails } = useAvatarDetails()

// Control groups are keyed by avatar id (see useControls.ts's loadControls/saveControls), so
// there's nothing meaningful to show or add to until VRChat's OSC is both alive and has told us
// which avatar is loaded.
const ready = computed(() => oscConnected.value && !!avatarDetails.value)

onMounted(() => api.ready())
onMounted(() => loadTheme())
</script>

<template>
  <UApp>
    <div class="flex max-h-dvh flex-col gap-4 overflow-auto p-4">
      <AppHeader />

      <div
        v-if="ready"
        class="flex flex-col gap-4"
      >
        <div class="flex flex-wrap justify-center gap-4">
          <ControlGroup
            v-for="controlGroup in controls"
            :key="controlGroup.id"
            :ui="{ body: 'p-4 sm:p-4 space-y-4' }"
            :control-group="controlGroup"
          />
        </div>

        <div class="flex justify-center">
          <UButton
            icon="i-lucide-plus"
            @click="addGroup"
          >
            Add Group
          </UButton>
        </div>
      </div>

      <UCard
        v-else
        class="mx-auto w-full max-w-lg"
      >
        <div class="flex flex-col items-center gap-3 py-4 text-center">
          <UIcon
            name="i-lucide-radio-tower"
            class="size-10 text-muted"
          />
          <div>
            <p class="font-medium">
              {{ oscConnected ? 'Waiting for an avatar' : 'VRChat not detected' }}
            </p>
            <p class="text-sm text-muted">
              {{
                oscConnected
                  ? "OSC is connected, but no avatar has been detected yet. Load into a world and it'll show up here."
                  : "No OSC data has been received yet. Make sure VRChat is running and OSC is enabled."
              }}
            </p>
          </div>

          <div
            v-if="!oscConnected"
            class="w-full space-y-1.5 rounded-md bg-elevated/50 p-3 text-left text-sm"
          >
            <p class="font-medium">
              How to enable OSC in VRChat
            </p>
            <ol class="list-decimal space-y-1 pl-5 text-muted">
              <li>Launch VRChat and load into any world.</li>
              <li>
                Enable OSC, either via
                <span class="text-default">Radial Menu → Options → OSC → Enabled</span>
                or via
                <span class="text-default">Settings → Avatars → OSC</span>.
              </li>
            </ol>
          </div>
        </div>
      </UCard>
    </div>
    <ControlModal />
    <LockedControlGroupModal />
    <LockedControlGroupListModal />
    <AreYouSureModal />
    <SettingsModal />
    <ClientsListSliderover />
    <ClientLogsModal />
  </UApp>
</template>

<style></style>
