<script setup lang="ts">
import { useAddons } from '@renderer/composables/useAddons'
import { useAiAccess } from '@renderer/composables/useAiAccess'
import { useAiSession } from '@renderer/composables/useAiSession'
import { uniqueClientCount } from '@renderer/composables/useWebsocketHost'
import { useLocalStorage } from '@vueuse/core'
import { computed, ref } from 'vue'
import { VueDraggable } from 'vue-draggable-plus'
import { useRoute, useRouter } from 'vue-router'

// The app's primary navigation, replacing the old header's icon buttons - one entry per page.
interface RailItem {
  key: string
  label: string
  icon: string
  to: string
  badge?: number
  busy?: boolean
  alert?: 'success' | 'error' | null
  // A page an add-on brings, shown only while it's on.
  addon?: boolean
}

const route = useRoute()
const router = useRouter()
const { access: aiAccess } = useAiAccess()
const { generating: aiGenerating, unseen: aiUnseen } = useAiSession()
const { isOn, addons, needsSetup } = useAddons()

// Expanded shows each page's name next to its icon; collapsed is icons only.
const collapsed = useLocalStorage('railCollapsed', false)

// The page order you dragged, as keys. Pages you never placed keep their default order, after yours.
const order = useLocalStorage<string[]>('railOrder', [])
const reordering = ref(false)

const items = computed<RailItem[]>(() => [
  { key: 'controls', label: 'Controls', icon: 'i-lucide-layout-grid', to: '/controls' },
  { key: 'presets', label: 'Presets', icon: 'i-lucide-layers', to: '/presets' },
  // Toy patterns come with the Intiface toys add-on.
  ...(isOn('intiface') ? [{ key: 'patterns', label: 'Patterns', icon: 'i-lucide-audio-waveform', to: '/patterns', addon: true }] : []),
  // So do avatar haptics, an add-on of their own.
  ...(isOn('haptics') ? [{ key: 'haptics', label: 'Haptics', icon: 'i-lucide-hand-heart', to: '/haptics', addon: true }] : []),
  ...(isOn('leash') ? [{ key: 'leash', label: 'Leash', icon: 'i-tkosc-collar', to: '/leash', addon: true }] : []),
  { key: 'automations', label: 'Automations', icon: 'i-lucide-workflow', to: '/automations' },
  { key: 'profiles', label: 'Profiles', icon: 'i-lucide-lock', to: '/profiles' },
  { key: 'viewers', label: 'Viewers', icon: 'i-lucide-users', to: '/viewers', badge: uniqueClientCount.value },
  { key: 'activity', label: 'Activity', icon: 'i-lucide-history', to: '/activity' },
  { key: 'parameters', label: 'Parameters', icon: 'i-lucide-list-tree', to: '/parameters' },
  ...(aiAccess.value.hasAccess
    ? [{ key: 'ai', label: 'AI', icon: 'i-lucide-sparkles', to: '/ai', busy: aiGenerating.value, alert: aiUnseen.value }]
    : [])
])

const rank = (key: string): number => {
  const index = order.value.indexOf(key)
  return index < 0 ? order.value.length : index
}

const shown = computed<RailItem[]>({
  get: () => [...items.value].sort((a, b) => rank(a.key) - rank(b.key)),
  set: (list) => {
    // Pages hidden right now (an add-on turned off) keep their saved place.
    const next = list.map((item) => item.key)
    order.value.forEach((key, index) => {
      if (!next.includes(key)) next.splice(index, 0, key)
    })
    order.value = next
  }
})

const menu = computed(() => [
  [
    { label: 'Reorder pages', icon: 'i-lucide-grip-vertical', disabled: reordering.value, onSelect: () => (reordering.value = true) },
    { label: 'Reset order', icon: 'i-lucide-rotate-ccw', disabled: !order.value.length, onSelect: () => (order.value = []) }
  ]
])

const settings: RailItem = { key: 'settings', label: 'Settings', icon: 'i-lucide-settings', to: '/settings' }
// Add-ons left for later in the welcome setup that still don't work.
const needingSetup = computed(() => addons.value.filter(needsSetup).length)

const isActive = (item: RailItem): boolean => route.path.startsWith(item.to)

const select = (item: RailItem): void => {
  if (!reordering.value) void router.push(item.to)
}
</script>

<template>
  <nav
    aria-label="Pages"
    class="flex min-h-0 shrink-0 flex-col border-r border-default"
    :class="collapsed ? 'w-17' : 'w-max min-w-50'"
  >
    <!-- The pages scroll when they don't fit, without a scrollbar; Settings stays at the bottom. -->
    <!-- Right-click opens Reorder pages / Reset order; while reordering, the pages drag instead of navigating. -->
    <UContextMenu :items="menu">
      <div class="flex min-h-0 flex-1 flex-col">
        <VueDraggable
          v-model="shown"
          :disabled="!reordering"
          :animation="200"
          :force-fallback="true"
          class="no-scrollbar flex min-h-0 flex-1 flex-col gap-1 overflow-y-auto p-2"
        >
          <RailButton
            v-for="item in shown"
            :key="item.key"
            :icon="item.icon"
            :label="item.label"
            :active="isActive(item)"
            :badge="item.badge"
            :busy="item.busy"
            :alert="item.alert"
            :addon="item.addon"
            :collapsed="collapsed"
            :reordering="reordering"
            @click="select(item)"
          />
        </VueDraggable>
      </div>
    </UContextMenu>

    <div class="flex shrink-0 flex-col gap-1 p-2 pt-1">
      <RailButton
        :icon="settings.icon"
        :label="settings.label"
        :active="isActive(settings)"
        :badge="needingSetup"
        :collapsed="collapsed"
        @click="select(settings)"
      />
      <RailButton
        v-if="reordering"
        icon="i-lucide-check"
        label="Done"
        active
        :collapsed="collapsed"
        @click="reordering = false"
      />
      <RailButton
        v-else
        :icon="collapsed ? 'i-lucide-panel-left-open' : 'i-lucide-panel-left-close'"
        :label="collapsed ? 'Expand' : 'Collapse'"
        :collapsed="collapsed"
        @click="collapsed = !collapsed"
      />
    </div>
  </nav>
</template>
