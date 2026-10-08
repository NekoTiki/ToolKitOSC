<script setup lang="ts">
import AppRail from '@renderer/components/layout/AppRail.vue'
import TopBar from '@renderer/components/layout/TopBar.vue'
import TitleBar from '@renderer/components/TitleBar.vue'
import { useAppUpdater } from '@renderer/composables/useAppUpdater'
import { useLogRetention } from '@renderer/composables/useLogRetention'
import { useStopEverythingTriggers } from '@renderer/composables/useStopEverything'
import { useTheme } from '@renderer/composables/useTheme'
import { api } from '@renderer/lib/tauri-bridge'
import { onMounted } from 'vue'

// The app shell: title bar, global top bar, then the navigation rail beside whichever page the
// router shows. Pages bring their own context column and header (see layout/PageLayout.vue).
const { loadTheme } = useTheme()

onMounted(() => api.ready())
onMounted(() => loadTheme())
useLogRetention()
useAppUpdater()
useStopEverythingTriggers()
</script>

<template>
  <UApp>
    <div class="flex h-dvh flex-col">
      <TitleBar />
      <TopBar />

      <div class="flex min-h-0 flex-1">
        <AppRail />
        <!-- Keyed by path for the editor, so going from one control's editor to another's starts a
        fresh draft; other pages keep their instance (the Controls page keeps its search when you
        switch groups). -->
        <RouterView v-slot="{ Component, route }">
          <component
            :is="Component"
            :key="route.name === 'controls' ? 'controls' : route.fullPath"
            class="min-w-0 flex-1"
          />
        </RouterView>
      </div>
    </div>
  </UApp>
</template>
