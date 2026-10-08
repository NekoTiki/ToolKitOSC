<script setup lang="ts">
import { useAiAccess } from '@renderer/composables/useAiAccess'
import { useAiSession } from '@renderer/composables/useAiSession'
import { uniqueClientCount } from '@renderer/composables/useWebsocketHost'
import { computed } from 'vue'
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
}

const route = useRoute()
const router = useRouter()
const { access: aiAccess } = useAiAccess()
const { generating: aiGenerating, unseen: aiUnseen } = useAiSession()

const items = computed<RailItem[]>(() => [
  { key: 'controls', label: 'Controls', icon: 'i-lucide-layout-grid', to: '/controls' },
  { key: 'presets', label: 'Presets', icon: 'i-lucide-layers', to: '/presets' },
  { key: 'automations', label: 'Automations', icon: 'i-lucide-workflow', to: '/automations' },
  { key: 'profiles', label: 'Profiles', icon: 'i-lucide-lock', to: '/profiles' },
  { key: 'viewers', label: 'Viewers', icon: 'i-lucide-users', to: '/viewers', badge: uniqueClientCount.value },
  { key: 'activity', label: 'Activity', icon: 'i-lucide-history', to: '/activity' },
  { key: 'parameters', label: 'Parameters', icon: 'i-lucide-list-tree', to: '/parameters' },
  ...(aiAccess.value.hasAccess
    ? [{ key: 'ai', label: 'AI', icon: 'i-lucide-sparkles', to: '/ai', busy: aiGenerating.value, alert: aiUnseen.value }]
    : [])
])

const settings: RailItem = { key: 'settings', label: 'Settings', icon: 'i-lucide-settings', to: '/settings' }

const isActive = (item: RailItem): boolean => route.path.startsWith(item.to)

const select = (item: RailItem): void => void router.push(item.to)
</script>

<template>
  <nav
    aria-label="Pages"
    class="flex min-h-0 w-17 shrink-0 flex-col gap-1 overflow-y-auto border-r border-default p-2"
  >
    <RailButton
      v-for="item in items"
      :key="item.key"
      :icon="item.icon"
      :label="item.label"
      :active="isActive(item)"
      :badge="item.badge"
      :busy="item.busy"
      :alert="item.alert"
      @click="select(item)"
    />

    <div class="flex-1" />

    <RailButton
      :icon="settings.icon"
      :label="settings.label"
      :active="isActive(settings)"
      @click="select(settings)"
    />
  </nav>
</template>
