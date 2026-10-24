<script setup lang="ts">
import IntifaceFields from '@renderer/components/control-modal/IntifaceFields.vue'
import SegmentedControl from '@renderer/components/ui/SegmentedControl.vue'
import { useIntiface } from '@renderer/composables/useIntiface'
import { allPatterns } from '@renderer/composables/useIntifacePatterns'
import type { ToyAction, ToyPatternAction } from '@renderer/lib/tauri-bridge'
import type { ConditionType, ToyRunType } from '@renderer/utils/automations'
import {
  defaultToyRun,
  formatMs,
  MAX_TOY_PATTERN_MS,
  MAX_TOY_PULSE_MS,
  TOY_RUNS,
  toyRunsFor
} from '@renderer/utils/automations'
import { computed, ref } from 'vue'

// The fields of the Toy and Play pattern steps: which toys, and how they run.
const props = defineProps<{ condition: ConditionType }>()
const step = defineModel<ToyAction | ToyPatternAction>({ required: true })

const { devices, knownDevices } = useIntiface()

// No toys picked means every toy connected when it runs.
const allToys = ref(step.value.toys.length === 0)
const setAllToys = (value: boolean): void => {
  allToys.value = value
  if (value) step.value.toys = []
}

// USlider can hand back an array; only plain numbers are taken (see OpenShockFields.vue).
const fromSlider = (value: number | number[] | undefined, apply: (value: number) => void): void => {
  if (typeof value === 'number') apply(value)
}
const toMs = (value: number | string, max: number): number => Math.min(max, Math.max(1, Math.round(Number(value))))
const pct = (level: number): string => `${Math.round(level * 100)} %`

const toy = computed(() => (step.value.kind === 'toy' ? step.value : null))
const pattern = computed(() => (step.value.kind === 'toy-pattern' ? step.value : null))

const runType = computed({
  get: (): ToyRunType => toy.value?.run.type ?? 'pulse',
  set: (type: ToyRunType) => {
    if (toy.value) toy.value.run = defaultToyRun(type)
  }
})
const runItems = computed(() => toyRunsFor(props.condition).map((type) => ({ value: type, label: TOY_RUNS[type].label })))

const patternItems = computed(() => allPatterns().map((p) => ({ value: p.id, label: p.name, icon: p.icon })))
</script>

<template>
  <div class="grid gap-4">
    <USwitch
      :model-value="allToys"
      label="All toys"
      description="Every toy connected when it runs."
      @update:model-value="setAllToys(!!$event)"
    />
    <IntifaceFields
      v-if="!allToys"
      v-model:actuators="step.toys"
      :devices="devices"
      :known-devices="knownDevices"
      :required="false"
      description="Pick the motors it drives. With none picked, it drives every toy."
    />

    <template v-if="toy">
      <UFormField
        v-if="runItems.length > 1"
        label="Action"
        :description="TOY_RUNS[runType].description"
      >
        <SegmentedControl
          v-model="runType"
          :items="runItems"
          aria-label="Action"
          class="w-fit max-w-full flex-wrap"
        />
      </UFormField>

      <div
        v-if="toy.run.type === 'pulse'"
        class="grid gap-3 sm:grid-cols-2"
      >
        <UFormField :label="`Level: ${pct(toy.run.level)}`">
          <USlider
            :model-value="toy.run.level"
            :min="0"
            :max="1"
            :step="0.01"
            @update:model-value="fromSlider($event, (value) => toy?.run.type === 'pulse' && (toy.run.level = value))"
          />
        </UFormField>
        <UFormField
          label="For"
          :description="`Milliseconds. Up to ${formatMs(MAX_TOY_PULSE_MS)}.`"
        >
          <UInput
            :model-value="toy.run.ms"
            type="number"
            min="1"
            :max="MAX_TOY_PULSE_MS"
            step="100"
            class="w-full"
            @update:model-value="toy.run.type === 'pulse' && (toy.run.ms = toMs($event, MAX_TOY_PULSE_MS))"
          />
        </UFormField>
      </div>

      <UFormField
        v-else-if="toy.run.type === 'hold'"
        :label="`While held: ${pct(toy.run.level)}`"
      >
        <USlider
          :model-value="toy.run.level"
          :min="0"
          :max="1"
          :step="0.01"
          @update:model-value="fromSlider($event, (value) => toy?.run.type === 'hold' && (toy.run.level = value))"
        />
      </UFormField>

      <div
        v-else-if="toy.run.type === 'ramp'"
        class="grid gap-3 sm:grid-cols-3"
      >
        <UFormField :label="`From: ${pct(toy.run.from)}`">
          <USlider
            :model-value="toy.run.from"
            :min="0"
            :max="1"
            :step="0.01"
            @update:model-value="fromSlider($event, (value) => toy?.run.type === 'ramp' && (toy.run.from = value))"
          />
        </UFormField>
        <UFormField :label="`To: ${pct(toy.run.to)}`">
          <USlider
            :model-value="toy.run.to"
            :min="0"
            :max="1"
            :step="0.01"
            @update:model-value="fromSlider($event, (value) => toy?.run.type === 'ramp' && (toy.run.to = value))"
          />
        </UFormField>
        <UFormField
          label="Over"
          :description="`Milliseconds. Up to ${formatMs(MAX_TOY_PULSE_MS)}.`"
        >
          <UInput
            :model-value="toy.run.ms"
            type="number"
            min="1"
            :max="MAX_TOY_PULSE_MS"
            step="100"
            class="w-full"
            @update:model-value="toy.run.type === 'ramp' && (toy.run.ms = toMs($event, MAX_TOY_PULSE_MS))"
          />
        </UFormField>
      </div>

      <div
        v-else
        class="grid gap-3 sm:grid-cols-2"
      >
        <UFormField :label="`At 0: ${pct(toy.run.min)}`">
          <USlider
            :model-value="toy.run.min"
            :min="0"
            :max="1"
            :step="0.01"
            @update:model-value="fromSlider($event, (value) => toy?.run.type === 'follow' && (toy.run.min = value))"
          />
        </UFormField>
        <UFormField :label="`At 1: ${pct(toy.run.max)}`">
          <USlider
            :model-value="toy.run.max"
            :min="0"
            :max="1"
            :step="0.01"
            @update:model-value="fromSlider($event, (value) => toy?.run.type === 'follow' && (toy.run.max = value))"
          />
        </UFormField>
      </div>
    </template>

    <template v-else-if="pattern">
      <UFormField label="Pattern">
        <USelect
          v-model="pattern.patternId"
          :items="patternItems"
          class="w-full max-w-80"
        />
      </UFormField>
      <UFormField
        v-if="!pattern.whileHeld"
        label="For"
        :description="`Milliseconds. Up to ${formatMs(MAX_TOY_PATTERN_MS)}.`"
      >
        <UInput
          :model-value="pattern.ms"
          type="number"
          min="1"
          :max="MAX_TOY_PATTERN_MS"
          step="1000"
          class="w-full max-w-40"
          @update:model-value="pattern && (pattern.ms = toMs($event, MAX_TOY_PATTERN_MS))"
        />
      </UFormField>
      <p
        v-else
        class="text-sm text-muted"
      >
        Plays while the condition is true, then stops.
      </p>
    </template>

    <p class="text-[13px] text-muted">
      Acts as you, so your profile's toy limits don't apply. Avatar touch still comes first.
    </p>
  </div>
</template>
