<script setup lang="ts">
import AboutSettings from '@renderer/components/settings/AboutSettings.vue'
import AccountSettings from '@renderer/components/settings/AccountSettings.vue'
import AppearanceSettings from '@renderer/components/settings/AppearanceSettings.vue'
import DataSettings from '@renderer/components/settings/DataSettings.vue'
import IntifaceSettings from '@renderer/components/settings/IntifaceSettings.vue'
import OpenShockSettings from '@renderer/components/settings/OpenShockSettings.vue'
import SharingSettings from '@renderer/components/settings/SharingSettings.vue'
import StartupSettings from '@renderer/components/settings/StartupSettings.vue'
import StopSettings from '@renderer/components/settings/StopSettings.vue'
import type { Component } from 'vue'
import { computed } from 'vue'
import { useRoute } from 'vue-router'

// Settings as a page with a section list, replacing the three-tab modal. Changes apply as you
// make them, like before - there's no Save button. `/settings/<section>` deep-links a section
// (the top bar's OpenShock and Intiface chips link to theirs).
const SECTIONS: { key: string; label: string; description: string; icon: string; component: Component }[] = [
  { key: 'appearance', label: 'Appearance', description: 'Theme colors and tile size', icon: 'i-lucide-palette', component: AppearanceSettings },
  { key: 'sharing', label: 'Sharing & access', description: 'Share link and who can use it', icon: 'i-lucide-share-2', component: SharingSettings },
  { key: 'stop', label: 'Stop everything', description: 'Hotkey and avatar parameter', icon: 'i-lucide-octagon-x', component: StopSettings },
  { key: 'account', label: 'Account & server', description: 'Discord sign-in and server', icon: 'i-lucide-globe', component: AccountSettings },
  { key: 'openshock', label: 'OpenShock', description: 'API key and shockers', icon: 'material-symbols:electric-bolt', component: OpenShockSettings },
  { key: 'intiface', label: 'Intiface', description: 'Connection and toys', icon: 'mdi:vibrate', component: IntifaceSettings },
  { key: 'startup', label: 'Startup', description: 'Tray, SteamVR and VRChat', icon: 'i-lucide-power', component: StartupSettings },
  { key: 'data', label: 'Data & logs', description: 'Activity log and bans', icon: 'i-lucide-database', component: DataSettings },
  { key: 'about', label: 'About & updates', description: 'Version and updates', icon: 'i-lucide-info', component: AboutSettings }
]

const route = useRoute()

const section = computed(() => SECTIONS.find((s) => s.key === route.params.section) ?? SECTIONS[0]!)
</script>

<template>
  <PageLayout>
    <template #sidebar>
      <div class="flex min-h-9 shrink-0 items-center">
        <b class="text-xl font-semibold text-highlighted">Settings</b>
      </div>
      <nav
        aria-label="Settings sections"
        class="-mr-1.5 flex min-h-0 flex-1 flex-col gap-1.5 overflow-y-auto pr-1.5"
      >
        <RouterLink
          v-for="item in SECTIONS"
          :key="item.key"
          :to="`/settings/${item.key}`"
          class="grid min-h-14.5 grid-cols-[auto_minmax(0,1fr)] items-center gap-x-2.5 rounded-2xl border px-2.5 py-2 transition-colors"
          :class="item.key === section.key ? 'border-primary/50 bg-primary/14' : 'border-transparent hover:bg-(--aurora-glass)'"
        >
          <span
            class="row-span-2 grid size-8.5 place-items-center rounded-xl bg-(--aurora-well)"
            :class="item.key === section.key ? 'text-primary' : 'text-muted'"
          >
            <UIcon
              :name="item.icon"
              class="size-4.5"
            />
          </span>
          <b class="truncate text-[14.5px] font-semibold text-highlighted">{{ item.label }}</b>
          <small class="col-start-2 truncate text-xs text-muted">{{ item.description }}</small>
        </RouterLink>
      </nav>
    </template>

    <template #header>
      <PageHeader
        :title="section.label"
        :crumbs="[{ label: 'Settings', to: '/settings' }, { label: section.label }]"
      />
    </template>

    <div class="grid max-w-3xl grid-cols-1 gap-3.5">
      <component :is="section.component" />
    </div>
  </PageLayout>
</template>
