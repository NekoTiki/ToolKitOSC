<script setup lang="ts">
import FormSection from '@renderer/components/ui/FormSection.vue'
import SegmentedControl from '@renderer/components/ui/SegmentedControl.vue'
import { useAddons } from '@renderer/composables/useAddons'
import type { OverlayNotify, OverlaySettings, OverlayTarget } from '@renderer/lib/tauri-bridge'
import { api } from '@renderer/lib/tauri-bridge'
import { computed, ref } from 'vue'

// Settings > VR overlay: toasts in XSOverlay or OVR Toolkit when viewers do something. Only you
// see them, unlike the chatbox.
const { getAddon } = useAddons()
const toast = useToast()
const settings = ref<OverlaySettings | null>(null)

void api.overlaySettings().then((value) => (settings.value = value))

const TARGETS: { value: OverlayTarget; label: string }[] = [
  { value: 'auto', label: 'Auto' },
  { value: 'xsoverlay', label: 'XSOverlay' },
  { value: 'ovrtoolkit', label: 'OVR Toolkit' },
  { value: 'off', label: 'Off' }
]

const EVENTS: { key: Exclude<keyof OverlayNotify, 'to'>; label: string; description: string }[] = [
  { key: 'shocks', label: 'Shocks and OWO', description: '"Sam fired Shocker · 40 % · 1.0 s". Never merged with anything else.' },
  { key: 'toys', label: 'Toys', description: '"Sam set Lush to 80 %"' },
  { key: 'controls', label: 'Other controls', description: '"Alex turned on Ears"' },
  { key: 'viewers', label: 'Viewers joining', description: '"Mika joined · 4 watching"' },
  { key: 'stop', label: 'Stop everything', description: '"Everything stopped" and "Controls resumed"' },
  { key: 'automations', label: 'Automations', description: '"Automation Headpat ran"' }
]

const status = computed(() => getAddon('overlay')?.status.text)

const change = (edit: (notify: OverlayNotify) => void): void => {
  if (!settings.value) return
  edit(settings.value.notify)
  void api.overlaySave(JSON.parse(JSON.stringify(settings.value))).catch((error) =>
    toast.add({ title: 'Could not save the VR overlay settings', description: String(error), color: 'error' })
  )
}

const sendTest = (): void => {
  api.overlayTestToast().catch((error) => toast.add({ title: 'Could not send a test toast', description: String(error), color: 'error' }))
}
</script>

<template>
  <template v-if="settings">
    <FormSection
      title="Send toasts to"
      :description="`Auto picks XSOverlay if it runs, otherwise OVR Toolkit. ${status ?? ''}`"
    >
      <SegmentedControl
        :model-value="settings.notify.to"
        :items="TARGETS"
        aria-label="Send toasts to"
        accent
        @update:model-value="(to) => change((notify) => (notify.to = to))"
      />
      <div>
        <UButton
          icon="i-lucide-send"
          color="neutral"
          variant="subtle"
          :disabled="settings.notify.to === 'off'"
          @click="sendTest"
        >
          Send a test toast
        </UButton>
      </div>
    </FormSection>

    <FormSection
      v-if="settings.notify.to !== 'off'"
      title="What sends a toast"
      description="At most one toast every 2 seconds. Actions that arrive in between are merged into one."
    >
      <USwitch
        v-for="event in EVENTS"
        :key="event.key"
        :model-value="settings.notify[event.key]"
        :label="event.label"
        :description="event.description"
        @update:model-value="change((notify) => (notify[event.key] = !!$event))"
      />
    </FormSection>
  </template>
</template>
