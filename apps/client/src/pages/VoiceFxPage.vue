<script setup lang="ts">
import CheckChip from '@renderer/components/ui/CheckChip.vue'
import ChoiceCard from '@renderer/components/ui/ChoiceCard.vue'
import EmptyState from '@renderer/components/ui/EmptyState.vue'
import FormSection from '@renderer/components/ui/FormSection.vue'
import SegmentedControl from '@renderer/components/ui/SegmentedControl.vue'
import { useControls } from '@renderer/composables/useControls'
import { useVoiceFx } from '@renderer/composables/useVoiceFx'
import type { VoiceFxDevices, VoiceFxHold } from '@renderer/lib/tauri-bridge'
import { api } from '@renderer/lib/tauri-bridge'
import { VOICE_FX_EFFECTS, voiceFxEffect } from '@toolkitosc/shared-ui'
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'

// Voice effects: your microphone goes through the app before VRChat hears it. Three tabs: what's
// held and what plays, the devices and how to wire them, and the mouth socket link.
const toast = useToast()
const { settings, state, change } = useVoiceFx()
const { controls } = useControls()

const tab = ref<'voice' | 'setup' | 'mouth'>('voice')
const TABS = [
  { value: 'voice' as const, label: 'Voice', icon: 'i-lucide-audio-lines' },
  { value: 'setup' as const, label: 'Setup', icon: 'i-lucide-cable' },
  { value: 'mouth' as const, label: 'Mouth socket', icon: 'i-lucide-link' }
]

const pct = (value: number): string => `${Math.round(value * 100)} %`

// --- Voice ---

const playing = computed(() => state.value.holds[0])
const mine = computed(() => state.value.holds.some((hold) => hold.source.kind === 'you'))

// Timed holds count down.
const now = ref(Date.now())
const clock = setInterval(() => (now.value = Date.now()), 500)

onBeforeUnmount(() => clearInterval(clock))

const controlName = (id: string): string =>
  controls.value.flatMap((group) => group.controls).find((control) => control.id === id)?.name ?? 'A control'

const who = (hold: VoiceFxHold): { name: string; via: string } => {
  switch (hold.source.kind) {
    case 'you':
      return { name: 'You', via: 'Turn on myself, on this page' }
    case 'avatar':
      return { name: 'Your avatar', via: 'TKOSC/VoiceFX' }
    case 'mouth':
      return { name: 'Mouth socket', via: state.value.mouthSockets.join(', ') || 'Your avatar' }
    case 'automation':
      return { name: 'Automation', via: 'A Voice effects step' }
    case 'control':
      return { name: hold.by ? `Viewer · ${hold.by}` : 'You', via: controlName(hold.source.id) }
  }
}

const left = (hold: VoiceFxHold): string => (hold.endsAt ? `${Math.max(0, Math.ceil((hold.endsAt - now.value) / 1000))} s left` : 'Until it lets go')

const failed = (title: string) => (error: unknown) => toast.add({ title, description: String(error), color: 'error', icon: 'i-lucide-circle-x' })

const holdMine = (on: boolean): void => void api.voicefxHold(on).catch(failed('Could not turn it on'))

const PARAMETERS = [
  { name: 'TKOSC/VoiceFX', way: 'IN', type: 'Bool', text: "While it's true, your avatar holds an effect on. False lets go of your avatar's hold only.", tip: 'Use a menu toggle, a contact or a grab.' },
  {
    name: 'TKOSC/VoiceFXEffect',
    way: 'IN',
    type: 'Int',
    text: "Which effect your avatar's hold uses: 0 default effect, 1 Muffled, 2 Underwater, 3 Radio, 4 Soft mute.",
    tip: 'Use one menu button per effect.'
  },
  {
    name: 'TKOSC/VoiceFXLevel',
    way: 'IN',
    type: 'Float',
    text: "The strength of your avatar's hold, from 0 to 1. Viewers and automations set their own.",
    tip: "Use a radial puppet, or a contact's proximity for a gag that tightens."
  },
  { name: 'TKOSC/VoiceFXOn', way: 'OUT', type: 'Bool', text: 'True while any effect plays, whoever started it. The app sets it; your avatar only reads it.', tip: 'Use it to show a gag, a mouth glow or an icon.' }
]

const explaining = ref(false)

// --- Setup ---

const DEFAULT_MIC = 'default'
const devices = ref<VoiceFxDevices>({ inputs: [], outputs: [] })
const loadingDevices = ref(false)

const loadDevices = async (): Promise<void> => {
  loadingDevices.value = true
  try {
    devices.value = await api.voicefxDevices()
  } catch (error) {
    failed('Could not list your audio devices')(error)
  } finally {
    loadingDevices.value = false
  }
}

onMounted(loadDevices)

// A saved device that's unplugged stays in its list, so the select still shows it.
const withSaved = (names: string[], saved: string | null | undefined): string[] => (saved && !names.includes(saved) ? [saved, ...names] : names)

const micItems = (saved: string | null | undefined): { label: string; value: string }[] => [
  { label: 'System default microphone', value: DEFAULT_MIC },
  ...withSaved(devices.value.inputs, saved).map((name) => ({ label: name, value: name }))
]
const outputItems = computed(() => withSaved(devices.value.outputs, settings.value?.output).map((name) => ({ label: name, value: name })))

const micValue = (value: string | undefined): string | null => (!value || value === DEFAULT_MIC ? null : value)

// The cable most people use, offered with one press when it's installed and nothing is picked.
const cable = computed(() => devices.value.outputs.find((name) => /cable input/i.test(name)))

const MIC_MODES = [
  { value: 'one' as const, label: 'One mic' },
  { value: 'split' as const, label: 'Desktop and VR' }
]

type Guide = 'cable' | 'voicemeeter' | 'wavelink'
const guide = ref<Guide>('cable')

const GUIDES: Record<Guide, { title: string; icon: string; description: string; steps: { title: string; text: string }[] }> = {
  cable: {
    title: 'VB-Audio Cable',
    icon: 'i-lucide-cable',
    description: 'The simplest: one free virtual cable.',
    steps: [
      { title: 'Install VB-CABLE', text: 'Get it from vb-audio.com/Cable, run the installer as administrator and restart your PC.' },
      { title: 'Pick your devices below', text: 'Microphone: your real mic. Output: CABLE Input.' },
      { title: "Set VRChat's mic", text: 'In VRChat, Settings > Audio > Microphone: CABLE Output.' },
      { title: 'Test it', text: 'Turn on Hear myself below, press Turn on myself on the Voice tab and talk.' },
      { title: 'Streaming?', text: 'Viewers hear the effect only if OBS captures CABLE Output instead of your mic.' }
    ]
  },
  voicemeeter: {
    title: 'VoiceMeeter',
    icon: 'i-lucide-sliders-vertical',
    description: 'You already mix your mic in VoiceMeeter.',
    steps: [
      { title: 'Send your mic to a bus', text: "In VoiceMeeter, turn on B1 on your mic's strip. That's what the app listens to." },
      { title: 'Pick your devices below', text: "Microphone: VoiceMeeter's B1 output. Output: a VoiceMeeter virtual input you don't use yet (AUX or VAIO3)." },
      { title: 'Route it back out', text: 'On that virtual input strip, turn on B2 only, so the effect never loops back into B1.' },
      { title: "Set VRChat's mic", text: "In VRChat, Settings > Audio > Microphone: VoiceMeeter's B2 output." },
      { title: 'Or keep it simple', text: 'Install VB-CABLE next to VoiceMeeter and use it as the output: then VRChat listens to CABLE Output.' }
    ]
  },
  wavelink: {
    title: 'Wave Link',
    icon: 'i-lucide-audio-waveform',
    description: 'You use an Elgato mic with Wave Link.',
    steps: [
      { title: 'Install VB-CABLE', text: 'Wave Link has no spare input to send the effect back through, so it needs the free cable too.' },
      { title: 'Pick your devices below', text: "Microphone: Wave Link's microphone output (with its own effects). Output: CABLE Input." },
      { title: "Set VRChat's mic", text: 'In VRChat, Settings > Audio > Microphone: CABLE Output.' },
      { title: 'Streaming?', text: 'Your Stream mix still has your clean mic. To let viewers hear the effect, add CABLE Output as an input in Wave Link and mute your mic in the Stream mix.' }
    ]
  }
}

// --- Mouth socket ---

const RELEASES = [
  { value: '0' as const, label: 'Right away' },
  { value: '1000' as const, label: 'After 1 s' },
  { value: '3000' as const, label: 'After 3 s' }
]
const release = computed({
  get: () => (settings.value?.mouth.releaseMs === 0 ? '0' : (settings.value?.mouth.releaseMs ?? 1000) >= 3000 ? '3000' : '1000'),
  set: (value) => change((target) => (target.mouth.releaseMs = Number(value)))
})

const SOCKET_MODES = [
  { value: 'auto' as const, label: 'Auto' },
  { value: 'custom' as const, label: 'Custom' }
]
// Ticked sockets this avatar doesn't have stay listed, so they can be unticked.
const socketNames = computed(() => [...new Set([...state.value.sockets.map((socket) => socket.name), ...(settings.value?.mouth.sockets ?? [])])])
const tickSocket = (name: string, on: boolean): void =>
  change((target) => (target.mouth.sockets = on ? [...target.mouth.sockets, name] : target.mouth.sockets.filter((other) => other !== name)))

const mouthDepth = computed(() => Math.max(0, ...state.value.sockets.filter((socket) => state.value.mouthSockets.includes(socket.name)).map((socket) => socket.depth)))
const mouthHold = computed(() => state.value.holds.find((hold) => hold.source.kind === 'mouth'))
</script>

<template>
  <PageLayout>
    <template #header>
      <PageHeader
        title="Voice effects"
        :meta="state.error ?? (state.running ? `${state.mic ?? 'Default microphone'} → ${settings?.output}` : undefined)"
      >
        <template
          v-if="settings"
          #actions
        >
          <UButton
            v-if="mine"
            icon="i-lucide-mic"
            color="neutral"
            variant="subtle"
            @click="holdMine(false)"
          >
            Let go
          </UButton>
          <UButton
            v-else
            icon="i-lucide-audio-lines"
            :disabled="state.paused"
            @click="holdMine(true)"
          >
            Turn on myself
          </UButton>
        </template>
      </PageHeader>
    </template>

    <EmptyState
      v-if="!settings"
      icon="i-lucide-loader-circle"
      title="Loading…"
      class="[&>span:first-child]:animate-spin"
    />

    <div
      v-else
      class="grid max-w-5xl gap-3.5"
    >
      <SegmentedControl
        v-model="tab"
        class="w-full *:flex-1"
        :items="TABS"
        accent
        aria-label="Voice effects sections"
      />

      <UAlert
        v-if="state.error"
        color="warning"
        variant="subtle"
        icon="i-lucide-triangle-alert"
        :title="state.error"
        :description="tab === 'setup' ? undefined : 'VRChat hears nothing through the app until this is fixed. See the Setup tab.'"
      />
      <UAlert
        v-if="state.paused"
        color="error"
        variant="subtle"
        icon="i-lucide-octagon-x"
        title="Stop everything is on"
        description="Every effect is off and none can start until you resume. Your voice still goes through, clean."
      />

      <!-- Voice: what plays, who holds what, the defaults and the avatar parameters. -->
      <div
        v-if="tab === 'voice'"
        class="grid grid-cols-[minmax(0,1.2fr)_minmax(0,1fr)] items-start gap-3.5 max-lg:grid-cols-1"
      >
        <div class="grid min-w-0 gap-3.5">
          <FormSection title="Now playing">
            <template #actions>
              <span class="text-xs text-muted">One effect at a time</span>
            </template>
            <div
              v-if="playing"
              class="flex items-center gap-4"
            >
              <span class="grid size-18 shrink-0 place-items-center rounded-field border border-primary/55 bg-primary/15 text-primary shadow-[0_0_22px_color-mix(in_oklab,var(--ui-primary)_30%,transparent)]">
                <UIcon
                  :name="voiceFxEffect(playing.effect).icon"
                  class="size-9"
                />
              </span>
              <span class="grid min-w-0 flex-1 gap-1">
                <b class="text-[28px] leading-tight font-semibold text-highlighted">{{ voiceFxEffect(playing.effect).label }}</b>
                <span class="text-[13px] text-muted">
                  From <b class="font-medium text-default">{{ who(playing).name }}</b> · {{ left(playing) }}
                </span>
              </span>
              <span class="grid justify-items-end">
                <span class="font-mono text-[10px] text-muted uppercase">Strength</span>
                <b class="text-2xl font-semibold text-highlighted tabular-nums">{{ pct(playing.level) }}</b>
              </span>
            </div>
            <div
              v-else
              class="flex items-center gap-3 rounded-field bg-(--aurora-well) p-4 text-muted"
            >
              <UIcon
                name="i-lucide-mic"
                class="size-7 shrink-0"
              />
              <span class="text-sm"><b class="font-semibold text-highlighted">Off.</b> Your clean voice goes through. Nothing is holding an effect.</span>
            </div>
          </FormSection>

          <FormSection
            title="Holds"
            description="Everything that wants an effect on right now. The newest one plays. When it ends, the next one still held takes over."
          >
            <div
              v-if="state.holds.length"
              class="grid gap-2"
            >
              <div
                v-for="(hold, index) in state.holds"
                :key="JSON.stringify(hold.source)"
                class="flex min-h-15 items-center gap-3 rounded-field bg-(--aurora-well) py-2 pr-2 pl-3.5"
              >
                <span class="w-5 text-xs font-semibold text-dimmed tabular-nums">#{{ index + 1 }}</span>
                <span class="grid min-w-0 flex-1 gap-0.5">
                  <b class="truncate text-sm font-medium text-highlighted">{{ who(hold).name }}</b>
                  <small class="truncate text-xs text-muted">{{ who(hold).via }}</small>
                </span>
                <span class="text-[13px] text-toned">{{ voiceFxEffect(hold.effect).label }} · {{ pct(hold.level) }}</span>
                <span class="min-w-16 text-right text-xs text-muted tabular-nums">{{ left(hold) }}</span>
                <UBadge
                  :color="hold.playing ? 'primary' : 'neutral'"
                  :variant="hold.playing ? 'solid' : 'outline'"
                >
                  {{ hold.playing ? 'Playing' : 'Waiting' }}
                </UBadge>
                <UButton
                  icon="i-lucide-x"
                  color="neutral"
                  variant="ghost"
                  aria-label="End this hold"
                  @click="api.voicefxRelease(hold.source).catch(failed('Could not end it'))"
                />
              </div>
            </div>
            <p
              v-else
              class="text-[13px] text-muted"
            >
              No holds. Your avatar, a viewer's control, an automation or you can start one.
            </p>
          </FormSection>
        </div>

        <div class="grid min-w-0 gap-3.5">
          <FormSection
            title="Default effect"
            description="Used when a source doesn't pick one: your avatar's toggle, Turn on myself, an automation without an effect."
          >
            <div class="grid grid-cols-2 gap-2.5">
              <ChoiceCard
                v-for="effect in VOICE_FX_EFFECTS"
                :key="effect.id"
                :title="effect.label"
                :description="effect.description"
                :icon="effect.icon"
                :selected="settings.defaultEffect === effect.id"
                @click="change((target) => (target.defaultEffect = effect.id))"
              />
            </div>
            <div class="grid gap-2">
              <div class="flex items-center justify-between gap-3 text-sm">
                <span class="text-muted">Strength</span>
                <b class="font-semibold text-highlighted tabular-nums">{{ pct(settings.level) }}</b>
              </div>
              <USlider
                :model-value="Math.round(settings.level * 100)"
                :min="0"
                :max="100"
                :step="1"
                size="lg"
                aria-label="Default strength"
                @update:model-value="change((target) => (target.level = ($event ?? 70) / 100))"
              />
              <small class="text-xs text-muted">For Turn on myself, and for your avatar until it sends TKOSC/VoiceFXLevel.</small>
            </div>
          </FormSection>

          <FormSection
            title="Avatar parameters"
            description="Add these to your avatar to drive the effect from your menu or a contact, and to show it on the avatar."
          >
            <template #actions>
              <UButton
                icon="i-lucide-circle-help"
                color="neutral"
                variant="subtle"
                @click="explaining = true"
              >
                How they work
              </UButton>
            </template>
            <div class="grid gap-1.5">
              <div
                v-for="parameter in PARAMETERS"
                :key="parameter.name"
                class="flex min-h-10 items-center gap-2.5"
              >
                <UBadge
                  :color="parameter.way === 'OUT' ? 'primary' : 'secondary'"
                  variant="outline"
                  class="w-11 justify-center"
                >
                  {{ parameter.way }}
                </UBadge>
                <code class="flex-1 font-mono text-[12.5px] text-toned">{{ parameter.name }}</code>
                <span class="text-xs text-muted">{{ parameter.type }}</span>
              </div>
            </div>
          </FormSection>
        </div>
      </div>

      <!-- Setup: how to wire it, and the devices. -->
      <div
        v-else-if="tab === 'setup'"
        class="grid grid-cols-2 items-start gap-3.5 max-lg:grid-cols-1"
      >
        <FormSection
          title="How is your audio set up?"
          description="The app listens to your mic, adds the effect and plays the result into a virtual cable. VRChat uses that cable as its microphone."
        >
          <div class="grid grid-cols-3 gap-2.5 max-sm:grid-cols-1">
            <ChoiceCard
              v-for="(option, key) in GUIDES"
              :key="key"
              :title="option.title"
              :description="option.description"
              :icon="option.icon"
              :selected="guide === key"
              @click="guide = key"
            />
          </div>
          <ol class="grid gap-2">
            <li
              v-for="(step, index) in GUIDES[guide].steps"
              :key="step.title"
              class="flex gap-3 rounded-field bg-(--aurora-well) px-3 py-2.5"
            >
              <span class="grid size-6 shrink-0 place-items-center rounded-full bg-primary/15 text-xs font-semibold text-primary">{{ index + 1 }}</span>
              <span class="grid min-w-0 gap-0.5">
                <b class="text-sm font-semibold text-highlighted">{{ step.title }}</b>
                <small class="text-[13px] leading-snug text-muted">{{ step.text }}</small>
              </span>
            </li>
          </ol>
        </FormSection>

        <div class="grid min-w-0 gap-3.5">
          <FormSection
            title="Devices"
            description="Change them here; your voice switches over right away."
          >
            <template #actions>
              <UButton
                icon="i-lucide-refresh-cw"
                color="neutral"
                variant="ghost"
                aria-label="Look for devices again"
                :loading="loadingDevices"
                @click="loadDevices"
              />
            </template>

            <div class="grid gap-1.5">
              <span class="text-[13px] text-toned">Microphone (what the app listens to)</span>
              <SegmentedControl
                :model-value="settings.micMode"
                class="w-full *:flex-1"
                :items="MIC_MODES"
                aria-label="Microphone mode"
                @update:model-value="change((target) => (target.micMode = $event))"
              />
            </div>
            <div
              class="grid gap-2.5"
              :class="{ 'grid-cols-2 max-sm:grid-cols-1': settings.micMode === 'split' }"
            >
              <label class="grid gap-1.5">
                <span class="flex items-center gap-2 text-[13px] text-toned">
                  {{ settings.micMode === 'split' ? 'On desktop' : 'Microphone' }}
                  <UBadge
                    v-if="settings.micMode === 'split' && state.running && (!state.vr || state.micFallback)"
                    size="sm"
                  >In use</UBadge>
                </span>
                <USelect
                  :model-value="settings.mic ?? DEFAULT_MIC"
                  :items="micItems(settings.mic)"
                  size="lg"
                  class="w-full"
                  @update:model-value="change((target) => (target.mic = micValue($event)))"
                />
              </label>
              <label
                v-if="settings.micMode === 'split'"
                class="grid gap-1.5"
              >
                <span class="flex items-center gap-2 text-[13px] text-toned">
                  In VR
                  <UBadge
                    v-if="state.running && state.vr && !state.micFallback"
                    size="sm"
                  >In use</UBadge>
                </span>
                <USelect
                  :model-value="settings.micVr ?? DEFAULT_MIC"
                  :items="micItems(settings.micVr)"
                  size="lg"
                  class="w-full"
                  @update:model-value="change((target) => (target.micVr = micValue($event)))"
                />
              </label>
            </div>
            <p
              v-if="settings.micMode === 'split'"
              class="text-xs leading-snug text-muted"
            >
              {{ state.vr ? "VRChat says you're in VR." : "VRChat says you're on desktop." }}
              {{ state.micFallback ? "The VR mic isn't there (headset off?), so the desktop one is used until it shows up." : '' }}
              It says so when you load in; until then the mic from last time is used.
            </p>

            <label class="grid gap-1.5">
              <span class="text-[13px] text-toned">Output (the cable VRChat listens to)</span>
              <USelect
                :model-value="settings.output ?? undefined"
                :items="outputItems"
                placeholder="Pick the cable"
                size="lg"
                class="w-full"
                @update:model-value="change((target) => (target.output = $event ?? null))"
              />
            </label>
            <UButton
              v-if="cable && !settings.output"
              icon="i-lucide-cable"
              variant="subtle"
              class="justify-self-start"
              @click="change((target) => (target.output = cable ?? null))"
            >
              Use {{ cable }}
            </UButton>

            <div class="grid gap-1.5">
              <span class="text-[13px] text-toned">Your voice right now</span>
              <div
                class="h-3 overflow-hidden rounded-full bg-(--aurora-well)"
                role="meter"
                aria-label="Microphone level"
                :aria-valuenow="Math.round(state.peak * 100)"
                aria-valuemin="0"
                aria-valuemax="100"
              >
                <div
                  class="h-full rounded-full bg-secondary transition-[width] duration-100"
                  :style="{ width: `${Math.min(100, state.peak * 140)}%` }"
                />
              </div>
            </div>

            <div class="flex items-center gap-3 rounded-field bg-(--aurora-well) px-3 py-2.5">
              <span class="grid min-w-0 flex-1">
                <b class="text-[14.5px] font-semibold text-highlighted">Hear myself</b>
                <small class="text-xs text-muted">Plays your voice in your own headphones, with the effect. For testing.</small>
              </span>
              <USwitch
                :model-value="settings.monitor"
                aria-label="Hear myself"
                @update:model-value="change((target) => (target.monitor = $event))"
              />
            </div>
          </FormSection>

          <UAlert
            color="warning"
            variant="subtle"
            icon="i-lucide-triangle-alert"
            title="If ToolKitOSC closes, VRChat hears nothing"
            description="Your voice goes through the app. To talk without it, set VRChat's microphone back to your real mic."
          />
        </div>
      </div>

      <!-- Mouth socket: something in the socket holds an effect, stronger the deeper it goes. -->
      <div
        v-else
        class="grid grid-cols-[minmax(0,1.2fr)_minmax(0,1fr)] items-start gap-3.5 max-lg:grid-cols-1"
      >
        <div class="grid min-w-0 gap-3.5">
          <FormSection
            title="Mouth socket"
            description="When something goes into your avatar's mouth socket, your voice gets an effect, stronger the deeper it goes. It reads the SPS, OGB or TPS socket already on the avatar: nothing new to add."
          >
            <template #actions>
              <USwitch
                :model-value="settings.mouth.enabled"
                aria-label="Mouth socket link"
                @update:model-value="change((target) => (target.mouth.enabled = $event))"
              />
            </template>

            <div class="grid gap-1.5">
              <span class="text-[13px] text-toned">Sockets</span>
              <SegmentedControl
                :model-value="settings.mouth.custom ? 'custom' : 'auto'"
                class="w-full *:flex-1"
                :items="SOCKET_MODES"
                aria-label="Sockets"
                @update:model-value="change((target) => (target.mouth.custom = $event === 'custom'))"
              />
              <div
                v-if="settings.mouth.custom && socketNames.length"
                class="flex flex-wrap gap-1.5"
              >
                <CheckChip
                  v-for="(name, index) in socketNames"
                  :id="`voicefx-socket-${index}`"
                  :key="name"
                  :model-value="settings.mouth.sockets.includes(name)"
                  :label="name"
                  @update:model-value="tickSocket(name, $event)"
                />
              </div>
              <small class="text-xs text-muted">
                <template v-if="state.mouthSockets.length">Following <b class="text-default">{{ state.mouthSockets.join(', ') }}</b>. The deepest sets the strength.</template>
                <template v-else-if="settings.mouth.custom">{{ socketNames.length ? 'Tick the sockets to follow.' : "This avatar's sockets show here once it loads." }}</template>
                <template v-else>This avatar has no socket named like a mouth (Blowjob, BJ, Mouth, Oral). Pick them in Custom.</template>
              </small>
            </div>

            <div class="grid gap-1.5">
              <span class="text-[13px] text-toned">Reacts to</span>
              <div class="flex flex-wrap gap-1.5">
                <CheckChip
                  id="voicefx-mouth-others"
                  :model-value="settings.mouth.others"
                  label="Other people"
                  @update:model-value="change((target) => (target.mouth.others = $event))"
                />
                <CheckChip
                  id="voicefx-mouth-own"
                  :model-value="settings.mouth.own"
                  label="Yourself"
                  @update:model-value="change((target) => (target.mouth.own = $event))"
                />
                <CheckChip
                  id="voicefx-mouth-hands"
                  :model-value="settings.mouth.hands"
                  label="Hands"
                  @update:model-value="change((target) => (target.mouth.hands = $event))"
                />
              </div>
            </div>
          </FormSection>

          <FormSection
            title="What it does"
            description="One effect, like every other source. Its strength follows the depth, between the two values below. Set both the same for a fixed strength."
          >
            <SegmentedControl
              :model-value="settings.mouth.effect"
              :items="VOICE_FX_EFFECTS.map((effect) => ({ value: effect.id, label: effect.label, icon: effect.icon }))"
              aria-label="Effect"
              @update:model-value="change((target) => (target.mouth.effect = $event))"
            />
            <div class="grid gap-2">
              <div class="flex items-center justify-between gap-3 text-sm">
                <span class="text-muted">At the tip</span>
                <b class="font-semibold text-highlighted tabular-nums">{{ pct(settings.mouth.tip) }}</b>
              </div>
              <USlider
                :model-value="Math.round(settings.mouth.tip * 100)"
                :min="0"
                :max="100"
                :step="1"
                size="lg"
                aria-label="Strength at the tip"
                @update:model-value="change((target) => (target.mouth.tip = ($event ?? 30) / 100))"
              />
            </div>
            <div class="grid gap-2">
              <div class="flex items-center justify-between gap-3 text-sm">
                <span class="text-muted">All the way in</span>
                <b class="font-semibold text-highlighted tabular-nums">{{ pct(settings.mouth.deep) }}</b>
              </div>
              <USlider
                :model-value="Math.round(settings.mouth.deep * 100)"
                :min="0"
                :max="100"
                :step="1"
                size="lg"
                aria-label="Strength all the way in"
                @update:model-value="change((target) => (target.mouth.deep = ($event ?? 100) / 100))"
              />
            </div>
            <div class="grid gap-1.5">
              <span class="text-[13px] text-toned">Lets go</span>
              <SegmentedControl
                v-model="release"
                :items="RELEASES"
                aria-label="Lets go"
              />
              <small class="text-xs text-muted">How long after it's fully out the effect ends. A short wait stops it flickering on every stroke.</small>
            </div>
          </FormSection>
        </div>

        <div class="grid min-w-0 gap-3.5">
          <FormSection title="Live">
            <div class="grid grid-cols-2 gap-2">
              <div class="grid gap-1.5 rounded-field bg-(--aurora-well) px-3 py-2.5">
                <span class="font-mono text-[10px] text-muted uppercase">Depth</span>
                <b class="text-xl font-semibold text-highlighted tabular-nums">{{ pct(mouthDepth) }}</b>
                <span class="h-1.5 overflow-hidden rounded-full bg-(--aurora-glass)"><span
                  class="block h-full rounded-full bg-secondary"
                  :style="{ width: `${mouthDepth * 100}%` }"
                /></span>
              </div>
              <div class="grid gap-1.5 rounded-field bg-(--aurora-well) px-3 py-2.5">
                <span class="font-mono text-[10px] text-muted uppercase">Strength</span>
                <b class="text-xl font-semibold text-highlighted tabular-nums">{{ mouthHold ? pct(mouthHold.level) : '—' }}</b>
                <span class="h-1.5 overflow-hidden rounded-full bg-(--aurora-glass)"><span
                  class="block h-full rounded-full bg-primary"
                  :style="{ width: `${(mouthHold?.level ?? 0) * 100}%` }"
                /></span>
              </div>
            </div>
            <p class="text-[13px] text-muted">
              {{
                !settings.mouth.enabled
                  ? 'The link is off.'
                  : mouthHold
                    ? mouthHold.playing
                      ? `${voiceFxEffect(mouthHold.effect).label} is on your voice.`
                      : 'Held, waiting behind a newer effect.'
                    : state.mouthSockets.length
                      ? 'Waiting for something in the socket.'
                      : 'Waiting for the socket to show up.'
              }}
            </p>
          </FormSection>

          <FormSection title="How it fits with the rest">
            <ul class="grid list-disc gap-1.5 pl-4.5 text-[13px] leading-snug text-default">
              <li>It starts a hold the moment something goes in. Being the newest, it plays over what was on, and that comes back when it lets go.</li>
              <li>Moving in and out only changes the strength. It doesn't count as a new hold.</li>
              <li><b>Stop everything</b> ends it. It starts again only once the socket is empty.</li>
              <li>Automations see it: When a voice effect turns on, started by the mouth socket.</li>
              <li>Viewers can't change it.</li>
            </ul>
          </FormSection>
        </div>
      </div>
    </div>

    <UModal
      v-model:open="explaining"
      title="How the avatar parameters work"
      description="Your avatar can turn an effect on like any other source, and it always knows when one is playing, whoever started it."
      :ui="{ content: 'sm:max-w-3xl' }"
    >
      <template #body>
        <div class="grid gap-4">
          <div class="grid gap-2">
            <div
              v-for="parameter in PARAMETERS"
              :key="parameter.name"
              class="flex flex-wrap items-baseline gap-x-3.5 gap-y-1.5 rounded-field bg-(--aurora-well) px-3.5 py-3"
            >
              <span class="flex w-52 shrink-0 items-center gap-2">
                <UBadge
                  :color="parameter.way === 'OUT' ? 'primary' : 'secondary'"
                  variant="outline"
                  class="w-11 justify-center"
                >{{ parameter.way }}</UBadge>
                <code class="font-mono text-[12.5px] text-highlighted">{{ parameter.name }}</code>
              </span>
              <span class="w-12 shrink-0 text-xs text-muted">{{ parameter.type }}</span>
              <span class="min-w-60 flex-1 text-[13px] leading-snug text-default">{{ parameter.text }} <span class="text-muted">{{ parameter.tip }}</span></span>
            </div>
          </div>

          <div class="grid gap-2">
            <b class="text-[15px] font-semibold text-highlighted">Setting them up</b>
            <ul class="grid list-disc gap-1 pl-4.5 text-[13px] leading-snug text-default">
              <li>Add the ones you use to your avatar's Expression Parameters, with the same names and types. Leave <b>Saved</b> off.</li>
              <li>Turn <b>Synced</b> on for TKOSC/VoiceFXOn if other players should see what it shows. The inputs can stay local.</li>
              <li>You don't need all four. TKOSC/VoiceFXOn alone shows the effect on your avatar; TKOSC/VoiceFX alone lets your menu turn it on.</li>
              <li>Several things can hold an effect at once. The newest plays; when it ends, the next one still held takes over.</li>
            </ul>
          </div>

          <UAlert
            color="warning"
            variant="subtle"
            icon="i-lucide-triangle-alert"
            title="Two things to know"
          >
            <template #description>
              <ul class="grid list-disc gap-1 pl-4">
                <li>Don't set TKOSC/VoiceFX from TKOSC/VoiceFXOn in your animator: the effect would keep itself on.</li>
                <li><b>Stop everything</b> ends every effect. To turn it on again from your avatar, switch your toggle off and on.</li>
              </ul>
            </template>
          </UAlert>
        </div>
      </template>
    </UModal>
  </PageLayout>
</template>
