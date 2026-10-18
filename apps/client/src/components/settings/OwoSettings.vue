<script setup lang="ts">
import OwoMuscles from '@renderer/components/owo/OwoMuscles.vue'
import type { StatusInfo } from '@renderer/components/settings/StatusLine.vue'
import StatusLine from '@renderer/components/settings/StatusLine.vue'
import FormSection from '@renderer/components/ui/FormSection.vue'
import { OWO_MUSCLES, useOwo } from '@renderer/composables/useOwo'
import type { OwoMuscle, OwoTesterEntry } from '@renderer/lib/tauri-bridge'
import { api } from '@renderer/lib/tauri-bridge'
import { computed, onUnmounted, ref, watch } from 'vue'

// Settings > OWO: the link to the OWO app, a check of the worn avatar and each muscle live, with
// its strength and Test each muscle.
const { settings, state, change, strength } = useOwo()
const toast = useToast()
// The OWO tester only exists in development builds: its Rust module isn't compiled into a
// release, the command refuses there, and this card isn't shown (see owo/simulator.rs).
const dev = import.meta.env.DEV

const simulate = (on: boolean): void => {
  api.owoSimulate(on).catch((error) => toast.add({ title: 'Could not switch the OWO tester', description: String(error), color: 'error' }))
}

// The tester's packets, newest first, while it's in use.
const log = ref<OwoTesterEntry[]>([])
let poll: ReturnType<typeof setInterval> | undefined

watch(
  () => dev && state.value.simulated,
  (on) => {
    clearInterval(poll)
    if (!on) return
    poll = setInterval(() => {
      api.owoTesterLog().then((entries) => (log.value = entries.slice(-30).reverse())).catch(() => {})
    }, 250)
  },
  { immediate: true }
)
onUnmounted(() => clearInterval(poll))

// Time since the packet before it, and whether that left the muscles without a sensation.
const gap = (index: number): { ms: number; late: boolean } | null => {
  const entry = log.value[index]
  const before = log.value[index + 1]

  if (!entry || !before || before.stop) return null

  const ms = entry.atMs - before.atMs
  return { ms, late: !entry.stop && ms > before.ms }
}

const statusInfo = computed<StatusInfo>(() => {
  switch (state.value.status) {
    case 'connecting':
      return { icon: 'i-lucide-circle-dashed', class: 'text-muted animate-pulse', label: 'Looking for the OWO app…' }
    case 'connected':
      return {
        icon: 'i-lucide-circle-check',
        class: 'text-success',
        label: state.value.simulated ? 'Connected to the OWO tester' : `Connected · ${state.value.address ?? ''}`
      }
    default:
      return { icon: 'i-lucide-circle-dashed', class: 'text-muted', label: 'Off' }
  }
})

const address = ref(settings.value?.address ?? '')
watch(
  () => settings.value?.address,
  (value) => (address.value = value ?? '')
)
const commitAddress = (): void => {
  const value = address.value.trim() || null
  if (value !== (settings.value?.address ?? null)) change((target) => (target.address = value))
}

const avatar = computed(() => state.value.avatar)
const missing = computed(() => (avatar.value ? 10 - avatar.value.muscles.length : 0))

const muscles = Object.keys(OWO_MUSCLES) as OwoMuscle[]

const setStrength = (muscle: OwoMuscle, value: number): void =>
  change((target) => {
    target.muscles[muscle] = { strength: value / 100 }
  })

const testMuscles = async (): Promise<void> => {
  try {
    await api.owoTestMuscles()
  } catch (error) {
    toast.add({ title: 'Could not test the muscles', description: String(error), color: 'error' })
  }
}
</script>

<template>
  <UAlert
    color="info"
    variant="subtle"
    icon="i-lucide-flask-conical"
    title="OWO is in beta"
    description="It hasn't been tried on a real OWO Skin yet, so the sensations and how they feel may still change."
  />

  <UAlert
    color="error"
    variant="subtle"
    icon="i-lucide-zap"
    title="The OWO Skin uses electrical pulses"
    description="Read the safety instructions in OWO's user guide, and calibrate your limits in the OWO app before you start. Those limits still apply under the strength set here."
    :actions="[{ label: 'OWO user guide', icon: 'i-lucide-external-link', color: 'error', variant: 'outline', onClick: () => void api.openUrl('https://owogame.com/docs/user-manual/') }]"
  />

  <section
    v-if="dev && settings"
    class="grid gap-3 rounded-panel border-2 border-dashed border-warning/60 bg-warning/8 p-4.5"
  >
    <div class="flex flex-wrap items-center gap-2.5">
      <UIcon
        name="i-lucide-test-tube-diagonal"
        class="size-5 text-warning"
      />
      <h2 class="flex-1 text-[17px] font-semibold text-highlighted">
        OWO tester
      </h2>
      <UBadge
        label="Dev build only"
        color="warning"
        variant="solid"
        class="rounded-full"
      />
    </div>
    <p class="text-[13px] leading-snug text-muted">
      A pretend OWO app on 127.0.0.1, so you can test without a suit. It shows every packet it gets. Only in development builds:
      released versions don't include it.
    </p>
    <USwitch
      :model-value="state.simulated"
      label="Use the tester"
      @update:model-value="simulate(!!$event)"
    />
    <ol
      v-if="state.simulated && log.length"
      class="grid max-h-72 gap-0.5 overflow-y-auto font-mono text-[11.5px] text-muted"
    >
      <li
        v-for="(entry, index) in log"
        :key="entry.atMs + entry.text"
        class="grid grid-cols-[4.5rem_4rem_minmax(0,1fr)] gap-2"
        :class="entry.stop ? 'text-warning' : undefined"
      >
        <span class="tabular-nums">{{ (entry.atMs / 1000).toFixed(2) }} s</span>
        <span
          class="tabular-nums"
          :class="gap(index)?.late ? 'text-error' : undefined"
        >{{ gap(index) ? `+${gap(index)!.ms} ms` : '' }}</span>
        <span class="break-all">{{ entry.text }}</span>
      </li>
    </ol>
  </section>

  <FormSection v-if="settings">
    <div class="flex flex-wrap items-center gap-2">
      <StatusLine :status="statusInfo" />
      <UBadge
        v-if="state.simulated"
        label="Tester"
        color="warning"
        variant="subtle"
        size="sm"
        class="rounded-full"
      />
    </div>
    <UAlert
      v-if="state.status === 'connecting' && !state.simulated"
      color="warning"
      variant="subtle"
      icon="i-lucide-radar"
      title="Connect ToolKitOSC in the OWO app"
      description="Open the OWO app on your PC or phone, go to Games › Scan game, and connect to it. Close vrc-owo-suit if it's running."
      :actions="dev ? [{ label: 'Use the tester', color: 'warning', variant: 'outline', onClick: () => simulate(true) }] : undefined"
    />

    <USwitch
      :model-value="settings.enabled"
      label="Avatar touch plays on your suit"
      description="Also TKOSC/OWO in your expression menu. Viewers and automations still play when it's off."
      @update:model-value="change((target) => (target.enabled = !!$event))"
    />
    <USwitch
      :model-value="settings.useAvatarIntensity"
      label="Use owo_intensity from the avatar"
      description="Your expression menu's radial scales touch. Only on avatars that have it."
      @update:model-value="change((target) => (target.useAvatarIntensity = !!$event))"
    />
    <UFormField
      label="OWO app address"
      description="Leave empty to find it on your network. Type its IP if a VPN or your router blocks that."
    >
      <UInput
        v-model="address"
        placeholder="Found automatically"
        class="w-full"
        autocomplete="off"
        :ui="{ base: 'font-mono' }"
        @change="commitAddress"
        @keyup.enter="commitAddress"
        @blur="commitAddress"
      />
    </UFormField>
  </FormSection>

  <FormSection
    v-if="avatar"
    :title="`Current avatar · ${avatar.name}`"
    :description="
      avatar.muscles.length
        ? `${avatar.muscles.length} / 10 muscles have a receiver${missing ? `. Add the other ${missing} with vrc-owo-suit's Unity package.` : '.'}`
        : 'This avatar has no OWO receivers. Add them with vrc-owo-suit\'s Unity package.'
    "
  >
    <p
      v-if="avatar.intensity || avatar.spsZones"
      class="text-[13px] text-muted"
    >
      {{ [avatar.intensity ? 'Has owo_intensity' : '', avatar.spsZones ? `${avatar.spsZones} SPS touch zone${avatar.spsZones === 1 ? '' : 's'}` : ''].filter(Boolean).join(' · ') }}
    </p>
  </FormSection>

  <FormSection
    title="Muscles"
    description="What each one plays right now. Strength is under the limits you calibrated in the OWO app."
  >
    <template #actions>
      <UButton
        icon="i-lucide-scan-line"
        color="neutral"
        variant="subtle"
        :loading="!!state.testing"
        :disabled="state.status !== 'connected'"
        @click="testMuscles"
      >
        Test each muscle
      </UButton>
    </template>
    <div class="flex flex-wrap items-end gap-4">
      <div
        v-for="side in ['front', 'back'] as const"
        :key="side"
        class="grid justify-items-center gap-1.5"
      >
        <OwoMuscles
          :side="side"
          :levels="state.playing"
        />
        <span class="text-xs text-muted">
          {{ side === 'front' ? 'Front' : 'Back' }}<template v-if="state.touched.includes(side)"> · being touched</template>
        </span>
      </div>
    </div>
    <div class="grid gap-x-6 gap-y-3 sm:grid-cols-2">
      <div
        v-for="muscle in muscles"
        :key="muscle"
        class="grid gap-1.5"
      >
        <div class="flex items-center gap-2 text-sm">
          <b class="font-semibold text-highlighted">{{ OWO_MUSCLES[muscle].label }}</b>
          <UBadge
            v-if="state.testing === muscle"
            color="warning"
            variant="subtle"
            size="sm"
          >
            Testing
          </UBadge>
          <span class="ml-auto font-semibold text-highlighted tabular-nums">{{ Math.round(strength(muscle) * 100) }}%</span>
        </div>
        <USlider
          :model-value="Math.round(strength(muscle) * 100)"
          :min="0"
          :max="100"
          :step="5"
          size="lg"
          :aria-label="`${OWO_MUSCLES[muscle].label} strength`"
          @update:model-value="setStrength(muscle, $event ?? 0)"
        />
      </div>
    </div>
  </FormSection>

  <FormSection
    title="Avatar parameters"
    description="Add this to your avatar to use it in VRChat."
  >
    <dl class="grid gap-2 text-sm">
      <div class="grid gap-0.5 rounded-field bg-(--aurora-well) px-3.5 py-2.5">
        <dt><code class="font-semibold text-highlighted">TKOSC/OWO</code> <span class="text-muted">· bool</span></dt>
        <dd class="text-muted">
          Turns avatar touch on and off from your expression menu, like <b class="text-highlighted">Avatar touch plays on your suit</b> here.
        </dd>
      </div>
    </dl>
  </FormSection>
</template>
