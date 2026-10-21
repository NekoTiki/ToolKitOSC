<script setup lang="ts">
import FormSection from '@renderer/components/ui/FormSection.vue'
import SegmentedControl from '@renderer/components/ui/SegmentedControl.vue'
import { useAddons } from '@renderer/composables/useAddons'
import type { OverlayCorner, OverlayHud, OverlayNotify, OverlaySettings, OverlayTarget } from '@renderer/lib/tauri-bridge'
import { api } from '@renderer/lib/tauri-bridge'
import { computed, ref } from 'vue'

// Settings > VR overlay: the panel on your wrist in SteamVR, the HUD in a corner of your view, and
// toasts in XSOverlay or OVR Toolkit when viewers do something. Only you see them, unlike the chatbox.
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

const HANDS: { value: 'left' | 'right'; label: string }[] = [
  { value: 'left', label: 'Left' },
  { value: 'right', label: 'Right' }
]

const CORNERS: { value: OverlayCorner; label: string }[] = [
  { value: 'bottom-left', label: 'Bottom left' },
  { value: 'bottom-right', label: 'Bottom right' },
  { value: 'top-left', label: 'Top left' },
  { value: 'top-right', label: 'Top right' }
]

const FOLLOWS: { value: OverlayHud['follow']; label: string }[] = [
  { value: 'lazy', label: 'Lazy' },
  { value: 'glued', label: 'Glued' }
]

type HudElement = 'status' | 'viewers' | 'lastAction' | 'toys' | 'leash' | 'warnings'

const HUD_ELEMENTS: { key: HudElement; label: string; description: string }[] = [
  { key: 'status', label: 'Sharing or Stopped', description: 'Whether viewers can use your controls right now' },
  { key: 'viewers', label: 'Viewers', description: 'How many are on your share page, and how many signed in' },
  { key: 'lastAction', label: 'Last action', description: '"Sam set Lush to 60 %" for a few seconds, then it fades' },
  { key: 'toys', label: 'Toys', description: "Each toy's level, amber while touch drives it" },
  { key: 'leash', label: 'Leash', description: 'Who pulls, where and how hard. Only while someone pulls.' },
  { key: 'warnings', label: 'Warnings', description: 'An add-on that needs you. Only while there is one.' }
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

// Saved a moment after the last change, so a dragged slider isn't written on every step.
// USlider can emit its value as a one-item array; Rust wants the number.
const slider = (value: number | number[] | undefined, fallback: number): number => (Array.isArray(value) ? value[0] : value) ?? fallback

let saveTimer: ReturnType<typeof setTimeout> | undefined

const change = (edit: (settings: OverlaySettings) => void): void => {
  if (!settings.value) return
  edit(settings.value)
  clearTimeout(saveTimer)
  saveTimer = setTimeout(() => {
    void api.overlaySave(JSON.parse(JSON.stringify(settings.value))).catch((error) =>
      toast.add({ title: 'Could not save the VR overlay settings', description: String(error), color: 'error' })
    )
  }, 250)
}

const sendTest = (): void => {
  api.overlayTestToast().catch((error) => toast.add({ title: 'Could not send a test toast', description: String(error), color: 'error' }))
}
</script>

<template>
  <template v-if="settings">
    <FormSection
      title="Wrist panel"
      :description="status"
    >
      <USwitch
        :model-value="settings.panel.enabled"
        label="Show ToolKitOSC on your wrist"
        description="While SteamVR runs. Turn the back of your hand toward you to see it, then poke it with your other hand's finger or point at it and pull the trigger. Stop is always at the bottom right."
        @update:model-value="change((s) => (s.panel.enabled = !!$event))"
      />
      <template v-if="settings.panel.enabled">
        <div class="flex flex-wrap items-center justify-between gap-3">
          <span class="text-sm text-highlighted">Wrist</span>
          <SegmentedControl
            :model-value="settings.panel.hand"
            :items="HANDS"
            aria-label="Wrist"
            accent
            @update:model-value="(hand) => change((s) => (s.panel.hand = hand))"
          />
        </div>
        <UFormField :label="`Size · ${settings.panel.widthCm} cm`">
          <USlider
            :model-value="settings.panel.widthCm"
            :min="10"
            :max="30"
            :step="1"
            size="lg"
            aria-label="Size"
            @update:model-value="change((s) => (s.panel.widthCm = slider($event, 16)))"
          />
        </UFormField>
        <UFormField :label="`Opacity · ${Math.round(settings.panel.opacity * 100)} %`">
          <USlider
            :model-value="Math.round(settings.panel.opacity * 100)"
            :min="30"
            :max="100"
            :step="5"
            size="lg"
            aria-label="Opacity"
            @update:model-value="change((s) => (s.panel.opacity = slider($event, 90) / 100))"
          />
        </UFormField>
        <UFormField
          :label="`Show when looking within · ${settings.panel.showWithinDeg}°`"
          description="How directly the back of your hand has to face you. It fades out over the next 15°."
        >
          <USlider
            :model-value="settings.panel.showWithinDeg"
            :min="15"
            :max="60"
            :step="5"
            size="lg"
            aria-label="Show when looking within"
            @update:model-value="change((s) => (s.panel.showWithinDeg = slider($event, 35)))"
          />
        </UFormField>
      </template>
    </FormSection>

    <FormSection
      title="HUD"
      description="A small display in a corner of your view. Nothing on it is pressed, so it never gets in your way. Avatar parameter TKOSC/Hud (bool) and the wrist panel's Home page show or hide it too."
    >
      <USwitch
        :model-value="settings.hud.enabled"
        label="Show the HUD"
        description="While SteamVR runs. Hidden while the SteamVR dashboard is open."
        @update:model-value="change((s) => (s.hud.enabled = !!$event))"
      />
      <template v-if="settings.hud.enabled">
        <div class="flex flex-wrap items-center justify-between gap-3">
          <span class="text-sm text-highlighted">Where</span>
          <SegmentedControl
            :model-value="settings.hud.corner"
            :items="CORNERS"
            aria-label="Where"
            accent
            @update:model-value="(corner) => change((s) => (s.hud.corner = corner))"
          />
        </div>
        <UFormField :label="`Size · ${Math.round(settings.hud.size * 100)} %`">
          <USlider
            :model-value="Math.round(settings.hud.size * 100)"
            :min="50"
            :max="200"
            :step="10"
            size="lg"
            aria-label="Size"
            @update:model-value="change((s) => (s.hud.size = slider($event, 100) / 100))"
          />
        </UFormField>
        <UFormField :label="`Opacity · ${Math.round(settings.hud.opacity * 100)} %`">
          <USlider
            :model-value="Math.round(settings.hud.opacity * 100)"
            :min="30"
            :max="100"
            :step="5"
            size="lg"
            aria-label="Opacity"
            @update:model-value="change((s) => (s.hud.opacity = slider($event, 85) / 100))"
          />
        </UFormField>
        <UFormField
          :label="`Distance · ${settings.hud.distance} m`"
          description="It keeps the same size in your view at any distance."
        >
          <USlider
            :model-value="settings.hud.distance"
            :min="0.5"
            :max="3"
            :step="0.25"
            size="lg"
            aria-label="Distance"
            @update:model-value="change((s) => (s.hud.distance = slider($event, 1.5)))"
          />
        </UFormField>
        <div class="flex flex-wrap items-center justify-between gap-3">
          <div>
            <p class="text-sm text-highlighted">
              Follow
            </p>
            <p class="text-sm text-muted">
              Lazy stays put while you look around a little, then glides back. Glued moves exactly with your view.
            </p>
          </div>
          <SegmentedControl
            :model-value="settings.hud.follow"
            :items="FOLLOWS"
            aria-label="Follow"
            accent
            @update:model-value="(follow) => change((s) => (s.hud.follow = follow))"
          />
        </div>
        <UFormField
          v-if="settings.hud.follow === 'lazy'"
          :label="`How fast it catches up · ${Math.round(settings.hud.catchUp * 100)} %`"
        >
          <USlider
            :model-value="Math.round(settings.hud.catchUp * 100)"
            :min="10"
            :max="100"
            :step="10"
            size="lg"
            aria-label="How fast it catches up"
            @update:model-value="change((s) => (s.hud.catchUp = slider($event, 50) / 100))"
          />
        </UFormField>
        <USwitch
          :model-value="settings.hud.hideWithPanel"
          label="Hide while the wrist panel shows"
          description="The panel shows the same things."
          @update:model-value="change((s) => (s.hud.hideWithPanel = !!$event))"
        />
      </template>
    </FormSection>

    <FormSection
      v-if="settings.hud.enabled"
      title="What the HUD shows"
      description="It shrinks to what's on it."
    >
      <USwitch
        v-for="element in HUD_ELEMENTS"
        :key="element.key"
        :model-value="settings.hud[element.key]"
        :label="element.label"
        :description="element.description"
        @update:model-value="change((s) => (s.hud[element.key] = !!$event))"
      />
    </FormSection>

    <FormSection
      title="Send toasts to"
      description="Auto picks XSOverlay if it runs, otherwise OVR Toolkit."
    >
      <SegmentedControl
        :model-value="settings.notify.to"
        :items="TARGETS"
        aria-label="Send toasts to"
        accent
        @update:model-value="(to) => change((s) => (s.notify.to = to))"
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
        @update:model-value="change((s) => (s.notify[event.key] = !!$event))"
      />
    </FormSection>
  </template>
</template>
