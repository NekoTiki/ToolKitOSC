<script setup lang="ts">
import PatternSparkline from '@renderer/components/patterns/PatternSparkline.vue'
import SegmentedControl from '@renderer/components/ui/SegmentedControl.vue'
import type { IntifacePatternDefinition } from '@renderer/composables/useIntifacePatterns'
import { allPatterns } from '@renderer/composables/useIntifacePatterns'
import type { IntifacePatternControl, ViewerPatternOptions } from '@toolkitosc/shared-ui'
import { DEFAULT_VIEWER_PATTERN_OPTIONS, PATTERN_MAX_MS } from '@toolkitosc/shared-ui'
import { computed } from 'vue'

const allowedPatterns = defineModel<IntifacePatternControl['allowedPatterns']>('allowedPatterns', {
  required: true
})

// Off unless the host turns it on. Turning it off keeps the options for next time.
const viewerPatterns = defineModel<ViewerPatternOptions | undefined>('viewerPatterns')

const viewerOptions = computed(() => ({ ...DEFAULT_VIEWER_PATTERN_OPTIONS, ...viewerPatterns.value }))

const setViewerOption = <K extends keyof ViewerPatternOptions>(key: K, value: ViewerPatternOptions[K]): void => {
  viewerPatterns.value = { ...viewerOptions.value, [key]: value }
}

// USlider can hand back an array; only plain numbers are taken (see OpenShockFields.vue).
const fromSlider = (value: number | number[] | undefined, apply: (value: number) => void): void => {
  if (typeof value === 'number') apply(value)
}

const STOPS_AFTER: { value: string; label: string; ms: number | null }[] = [
  { value: '1', label: '1 min', ms: 60_000 },
  { value: '5', label: '5 min', ms: 5 * 60_000 },
  { value: '15', label: '15 min', ms: 15 * 60_000 },
  { value: 'never', label: 'Never', ms: null }
]

const stopsAfter = computed({
  get: () => STOPS_AFTER.find((option) => option.ms === viewerOptions.value.stopsAfterMs)?.value ?? '5',
  set: (value: string) => setViewerOption('stopsAfterMs', STOPS_AFTER.find((option) => option.value === value)?.ms ?? null)
})

const who = computed({
  get: () => (viewerOptions.value.signedInOnly ? 'signed-in' : 'everyone'),
  set: (value: string) => setViewerOption('signedInOnly', value === 'signed-in')
})

// Your recorded patterns first, then the built-in ones.
const patterns = computed(() => allPatterns())

const isAllowed = (id: string): boolean =>
  allowedPatterns.value.some((pattern) => pattern.id === id)

// Only id/name/icon are kept on the control (see IntifacePatternRef in controls.ts) - the actual
// waveform (IntifacePatternDefinition.value) stays purely a client-side implementation detail this
// control never needs to carry around.
const toggle = (pattern: IntifacePatternDefinition, checked: boolean): void => {
  if (checked) {
    if (isAllowed(pattern.id)) return

    allowedPatterns.value = [
      ...allowedPatterns.value,
      { id: pattern.id, name: pattern.name, icon: pattern.icon }
    ]
  } else {
    allowedPatterns.value = allowedPatterns.value.filter((allowed) => allowed.id !== pattern.id)
  }
}
</script>

<template>
  <UFormField
    label="Patterns"
    required
    description="Pick which patterns viewers can choose from. Your patterns come first, then the built-in ones. 'Off' is always there, so anyone can stop the toy."
  >
    <template #hint>
      <ULink
        to="/patterns"
        class="text-sm text-primary"
      >
        Manage patterns →
      </ULink>
    </template>
    <div class="grid gap-2 sm:grid-cols-2">
      <!-- Explicit, per-checkbox id: see IntifaceFields.vue's actuator checkboxes for why one of
      these sitting inside a single UFormField needs its own id rather than none at all. -->
      <label
        v-for="(pattern, index) in patterns"
        :key="pattern.id"
        :for="`intiface-pattern-${index}`"
        class="flex min-h-12 cursor-pointer items-center gap-3 rounded-2xl border border-default bg-(--aurora-well) px-3"
      >
        <UCheckbox
          :id="`intiface-pattern-${index}`"
          :model-value="isAllowed(pattern.id)"
          @update:model-value="toggle(pattern, !!$event)"
        />
        <PatternSparkline
          :pattern="pattern"
          :mine="!!pattern.durationMs"
        />
        <span
          class="flex-1 truncate"
          :class="isAllowed(pattern.id) ? 'text-highlighted' : 'text-muted'"
        >{{ pattern.name }}</span>
        <UBadge
          :label="pattern.durationMs ? 'Mine' : 'Built-in'"
          :color="pattern.durationMs ? 'primary' : 'neutral'"
          variant="soft"
          size="sm"
        />
      </label>
    </div>
  </UFormField>

  <div class="mt-4 flex flex-col gap-4 rounded-2xl border border-default bg-(--aurora-well) p-4">
    <div class="flex items-center gap-3.5">
      <div class="flex-1">
        <b class="block font-semibold text-highlighted">Viewers can make their own pattern</b>
        <p class="text-sm text-muted">
          Adds a <b class="text-primary">Make one</b> button to the tile. Viewers record a pattern by moving a slider, and the toy
          follows them live. It isn't saved: it repeats until someone picks another pattern or the time runs out.
        </p>
      </div>
      <USwitch
        :model-value="!!viewerPatterns?.enabled"
        size="xl"
        aria-label="Viewers can make their own pattern"
        @update:model-value="setViewerOption('enabled', $event)"
      />
    </div>

    <template v-if="viewerPatterns?.enabled">
      <div class="grid gap-x-7 gap-y-4 sm:grid-cols-2">
        <UFormField :label="`Highest intensity: ${Math.round(viewerOptions.maxIntensity * 100)}%`">
          <USlider
            :model-value="viewerOptions.maxIntensity"
            :min="0.05"
            :max="1"
            :step="0.05"
            @update:model-value="fromSlider($event, (value) => setViewerOption('maxIntensity', value))"
          />
        </UFormField>
        <UFormField :label="`Longest recording: ${viewerOptions.maxMs / 1000} s`">
          <USlider
            :model-value="viewerOptions.maxMs / 1000"
            :min="1"
            :max="PATTERN_MAX_MS / 1000"
            :step="1"
            @update:model-value="fromSlider($event, (value) => setViewerOption('maxMs', value * 1000))"
          />
        </UFormField>
        <UFormField label="Stops after">
          <SegmentedControl
            v-model="stopsAfter"
            accent
            aria-label="Stops after"
            :items="STOPS_AFTER"
          />
        </UFormField>
        <UFormField label="Who can make one">
          <SegmentedControl
            v-model="who"
            accent
            aria-label="Who can make one"
            :items="[
              { value: 'everyone', label: 'Everyone' },
              { value: 'signed-in', label: 'Signed-in viewers' }
            ]"
          />
        </UFormField>
      </div>
      <p class="text-sm text-dimmed">
        The active profile's limits for this control still apply on top of these.
      </p>
    </template>
  </div>
</template>

<style scoped></style>
