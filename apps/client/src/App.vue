<script setup lang="ts">
import AppHeader from '@renderer/components/AppHeader.vue'
import AreYouSureModal from '@renderer/components/AreYouSureModal.vue'
import ControlGroup from '@renderer/components/controls/ControlGroup.vue'
import SettingsModal from '@renderer/components/SettingsModal.vue'
import { useControls } from '@renderer/composables/useControls'
import { useTheme } from '@renderer/composables/useTheme'
import { api } from '@renderer/lib/tauri-bridge'
import { onMounted } from 'vue'

const { controls } = useControls()
const { loadTheme } = useTheme()

onMounted(() => api.ready())
onMounted(() => loadTheme())
</script>

<template>
  <UApp>
    <div class="flex max-h-dvh flex-col gap-4 overflow-auto p-4">
      <AppHeader />

      <div
        class="flex flex-wrap justify-center gap-4"
      >
        <ControlGroup
          v-for="controlGroup in controls"
          :key="controlGroup.id"
          :ui="{ body: 'p-4 sm:p-4 space-y-4' }"
          :control-group="controlGroup"
        />
      </div>
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
