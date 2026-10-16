<script setup lang="ts">
import AddonOff from '@renderer/components/AddonOff.vue'
import AddonsFirstRun from '@renderer/components/AddonsFirstRun.vue'
import AppRail from '@renderer/components/layout/AppRail.vue'
import TopBar from '@renderer/components/layout/TopBar.vue'
import TitleBar from '@renderer/components/TitleBar.vue'
import { useAddons } from '@renderer/composables/useAddons'
import { useAppUpdater } from '@renderer/composables/useAppUpdater'
import { useAutomationRunner } from '@renderer/composables/useAutomationRunner'
import { useLogRetention } from '@renderer/composables/useLogRetention'
import { useStopEverythingTriggers } from '@renderer/composables/useStopEverything'
import { useTheme } from '@renderer/composables/useTheme'
import { api } from '@renderer/lib/tauri-bridge'
import { onMounted } from 'vue'

// The app shell: title bar, global top bar, then the navigation rail beside whichever page the
// router shows. Pages bring their own context column and header (see layout/PageLayout.vue).
const { loadTheme } = useTheme()
const { isOn, loaded: addonsLoaded, state: addonsState } = useAddons()

onMounted(() => api.ready())
onMounted(() => loadTheme())
useLogRetention()
useAppUpdater()
useStopEverythingTriggers()
useAutomationRunner()
</script>

<template>
  <UApp>
    <div class="flex h-dvh flex-col">
      <TitleBar />
      <TopBar />

      <!-- A new install asks "What do you use?" once, before the app itself. -->
      <AddonsFirstRun
        v-if="addonsState && !addonsState.setupDone"
        class="min-h-0 flex-1"
      />
      <div
        v-else
        class="flex min-h-0 flex-1"
      >
        <AppRail />
        <!-- Keyed by path for the editor, so going from one control's editor to another's starts a
        fresh draft; other pages keep their instance (the Controls page keeps its search when you
        switch groups). -->
        <RouterView v-slot="{ Component, route }">
          <!-- A page of an add-on that's off (e.g. Patterns while Intiface toys is off). -->
          <AddonOff
            v-if="route.meta.addon && addonsLoaded && !isOn(route.meta.addon)"
            :addon-id="route.meta.addon"
            class="min-w-0 flex-1"
          />
          <component
            :is="Component"
            v-else
            :key="route.name === 'controls' ? 'controls' : route.fullPath"
            class="min-w-0 flex-1"
          />
        </RouterView>
      </div>
    </div>
  </UApp>
</template>
