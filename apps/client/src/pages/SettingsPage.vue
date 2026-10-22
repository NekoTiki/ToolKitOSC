<script setup lang="ts">
import AddonOff from '@renderer/components/AddonOff.vue'
import AboutSettings from '@renderer/components/settings/AboutSettings.vue'
import AccountSettings from '@renderer/components/settings/AccountSettings.vue'
import AddonsSettings from '@renderer/components/settings/AddonsSettings.vue'
import AppearanceSettings from '@renderer/components/settings/AppearanceSettings.vue'
import BhapticsSettings from '@renderer/components/settings/BhapticsSettings.vue'
import BoardsSettings from '@renderer/components/settings/BoardsSettings.vue'
import ChatboxSettings from '@renderer/components/settings/ChatboxSettings.vue'
import CreditsSettings from '@renderer/components/settings/CreditsSettings.vue'
import DataSettings from '@renderer/components/settings/DataSettings.vue'
import IntifaceSettings from '@renderer/components/settings/IntifaceSettings.vue'
import OpenShockSettings from '@renderer/components/settings/OpenShockSettings.vue'
import OverlaySettings from '@renderer/components/settings/OverlaySettings.vue'
import OwoSettings from '@renderer/components/settings/OwoSettings.vue'
import SharingSettings from '@renderer/components/settings/SharingSettings.vue'
import StartupSettings from '@renderer/components/settings/StartupSettings.vue'
import StopSettings from '@renderer/components/settings/StopSettings.vue'
import { useAddons } from '@renderer/composables/useAddons'
import type { Component } from 'vue'
import { computed, ref, watch } from 'vue'
import { useRoute } from 'vue-router'

// Settings as a page with a section list, replacing the three-tab modal. Changes apply as you
// make them, like before - there's no Save button. `/settings/<section>` deep-links a section
// (the top bar's OpenShock and Intiface chips link to theirs). An add-on's section (`addon`) is only
// listed while that add-on is on, under Your add-ons. A section's sub-pages
// (`/settings/<section>/<sub>`, e.g. About › Credits) open in the same place, so the list stays.
interface Section {
  key: string
  label: string
  description: string
  icon: string
  component: Component
  addon?: string
  // Takes the page's whole width instead of a reading column, e.g. a grid of cards. Its sub-pages too.
  wide?: boolean
  subpages?: Record<string, { label: string; component: Component }>
}

const SECTIONS: Section[] = [
  { key: 'addons', label: 'Add-ons', description: 'Turn on only what you own', icon: 'i-lucide-puzzle', component: AddonsSettings, wide: true },
  { key: 'appearance', label: 'Appearance', description: 'Theme colors and tile size', icon: 'i-lucide-palette', component: AppearanceSettings },
  { key: 'sharing', label: 'Sharing & access', description: 'Share link and who can use it', icon: 'i-lucide-share-2', component: SharingSettings },
  { key: 'chatbox', label: 'VRChat chatbox', description: 'Post viewer actions in-game', icon: 'i-lucide-message-square-text', component: ChatboxSettings },
  { key: 'stop', label: 'Stop everything', description: 'Hotkey and avatar parameter', icon: 'i-lucide-octagon-x', component: StopSettings },
  { key: 'account', label: 'Account & server', description: 'Discord sign-in and server', icon: 'i-lucide-globe', component: AccountSettings },
  { key: 'startup', label: 'Startup', description: 'Tray, SteamVR and VRChat', icon: 'i-lucide-power', component: StartupSettings },
  { key: 'data', label: 'Data & logs', description: 'Activity log and bans', icon: 'i-lucide-database', component: DataSettings },
  { key: 'about', label: 'About & updates', description: 'Version and updates', icon: 'i-lucide-info', component: AboutSettings, wide: true, subpages: { credits: { label: 'Credits', component: CreditsSettings } } },
  { key: 'openshock', label: 'OpenShock', description: 'API key and shockers', icon: 'material-symbols:electric-bolt', component: OpenShockSettings, addon: 'openshock' },
  { key: 'intiface', label: 'Intiface', description: 'Connection and toys', icon: 'mdi:vibrate', component: IntifaceSettings, addon: 'intiface' },
  { key: 'bhaptics', label: 'bHaptics', description: 'Player and your gear · beta', icon: 'i-lucide-shirt', component: BhapticsSettings, addon: 'bhaptics' },
  { key: 'owo', label: 'OWO', description: 'The OWO app and your suit · beta', icon: 'i-lucide-zap', component: OwoSettings, addon: 'owo' },
  { key: 'esp32', label: 'ESP32 boards', description: 'Boards and outputs', icon: 'i-lucide-cpu', component: BoardsSettings, addon: 'boards' },
  { key: 'vr-overlay', label: 'VR overlay', description: 'Wrist panel and toasts · experimental', icon: 'i-lucide-glasses', component: OverlaySettings, addon: 'overlay' }
]

const route = useRoute()
const { isOn, getAddon } = useAddons()

const coreSections = SECTIONS.filter((s) => !s.addon)
const addonSections = computed(() => SECTIONS.filter((s) => s.addon && isOn(s.addon)))

const navGroups = computed(() => [
  { label: null, items: coreSections },
  ...(addonSections.value.length ? [{ label: 'Your add-ons', items: addonSections.value }] : [])
])

// A link to an add-on's section still opens it while the add-on is off, with a Turn on button.
const section = computed(() => SECTIONS.find((s) => s.key === route.params.section) ?? SECTIONS[0]!)
const subpage = computed(() => section.value.subpages?.[String(route.params.sub)] ?? null)
// The page stays mounted across sections (see App.vue), so the list keeps its scroll; the
// section itself starts at the top.
const body = ref<HTMLElement | null>(null)

watch(
  () => [section.value.key, subpage.value],
  () => body.value?.closest('.overflow-auto')?.scrollTo({ top: 0 })
)

const sectionOff = computed(() => (section.value.addon && !isOn(section.value.addon) ? section.value.addon : null))

const statusDot = (addonId: string): string => {
  const state = getAddon(addonId)?.status.state

  return state === 'ok' ? 'bg-success' : state === 'error' ? 'bg-error' : state === 'off' ? 'bg-(--ui-text-dimmed)' : 'bg-warning'
}
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
        <template
          v-for="navGroup in navGroups"
          :key="navGroup.label ?? 'core'"
        >
          <span
            v-if="navGroup.label"
            class="px-2.5 pt-3 font-mono text-[11.5px] tracking-widest text-muted uppercase"
          >{{ navGroup.label }}</span>
          <RouterLink
            v-for="item in navGroup.items"
            :key="item.key"
            :to="`/settings/${item.key}`"
            class="grid min-h-14.5 grid-cols-[auto_minmax(0,1fr)] items-center gap-x-2.5 rounded-2xl border px-2.5 py-2 transition-colors"
            :class="item.key === section.key ? 'border-primary/50 bg-primary/14' : 'border-transparent hover:bg-(--aurora-glass)'"
          >
            <span
              class="relative row-span-2 grid size-8.5 place-items-center rounded-xl bg-(--aurora-well)"
              :class="item.key === section.key ? 'text-primary' : 'text-muted'"
            >
              <UIcon
                :name="item.icon"
                class="size-4.5"
              />
              <span
                v-if="item.addon"
                class="absolute -top-0.5 -right-0.5 size-2.5 rounded-full ring-2 ring-(--ui-bg)"
                :class="statusDot(item.addon)"
              />
            </span>
            <b class="truncate text-[14.5px] font-semibold text-highlighted">{{ item.label }}</b>
            <small class="col-start-2 truncate text-xs text-muted">{{ item.addon ? getAddon(item.addon)?.status.text : item.description }}</small>
          </RouterLink>
        </template>
      </nav>
    </template>

    <template #header>
      <PageHeader
        :title="subpage?.label ?? section.label"
        :crumbs="
          subpage
            ? [{ label: 'Settings', to: '/settings' }, { label: section.label, to: `/settings/${section.key}` }, { label: subpage.label }]
            : [{ label: 'Settings', to: '/settings' }, { label: section.label }]
        "
      />
    </template>

    <div
      ref="body"
      class="grid grid-cols-1 gap-3.5"
      :class="section.wide && !sectionOff ? undefined : 'max-w-3xl'"
    >
      <AddonOff
        v-if="sectionOff"
        :addon-id="sectionOff"
      />
      <component
        :is="subpage?.component ?? section.component"
        v-else
      />
    </div>
  </PageLayout>
</template>
