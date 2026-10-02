<script setup lang="ts">
import FormSection from '@renderer/components/ui/FormSection.vue'
import SegmentedControl from '@renderer/components/ui/SegmentedControl.vue'
import { useControlsView } from '@renderer/composables/useControlsView'
import type { ColorFamilyWithoutNeutral } from '@renderer/composables/useTheme'
import { COLOR_FAMILIES, useTheme } from '@renderer/composables/useTheme'
import { ControlBase, ControlBooleanBase, ControlOptions, ControlSliderBase } from '@vrc-osc-toolkit/shared-ui'
import { computed, ref } from 'vue'

// The theme is two colors (see shared-ui's styles/aurora.css): primary for buttons, the selected
// page and anything switched on; secondary for values and the second background glow. Presets
// pick a matching pair at once. Viewers get the same theme on the share page.
const PRESETS: { name: string; primary: ColorFamilyWithoutNeutral; secondary: ColorFamilyWithoutNeutral }[] = [
  { name: 'Aurora', primary: 'teal', secondary: 'orange' },
  { name: 'Sakura', primary: 'pink', secondary: 'violet' },
  { name: 'Lagoon', primary: 'cyan', secondary: 'blue' },
  { name: 'Ember', primary: 'amber', secondary: 'rose' },
  { name: 'Mint', primary: 'emerald', secondary: 'sky' },
  { name: 'Nebula', primary: 'violet', secondary: 'fuchsia' },
  { name: 'Citrus', primary: 'lime', secondary: 'yellow' },
  { name: 'Ruby', primary: 'rose', secondary: 'indigo' }
]

// What Nuxt UI uses until the user picks something (see vite.config.ts).
const DEFAULT_PRIMARY: ColorFamilyWithoutNeutral = 'teal'
const DEFAULT_SECONDARY: ColorFamilyWithoutNeutral = 'orange'

const { selectedPrimary, selectedSecondary, setTheme } = useTheme()
const { density } = useControlsView()

const primary = computed(() => selectedPrimary.value ?? DEFAULT_PRIMARY)
const secondary = computed(() => selectedSecondary.value ?? DEFAULT_SECONDARY)

const applyPreset = (preset: (typeof PRESETS)[number]): void => {
  setTheme('primary', preset.primary)
  setTheme('secondary', preset.secondary)
}

const swatch = (family: string, shade = 500): string => `var(--color-${family}-${shade})`

const presetBackground = (preset: (typeof PRESETS)[number]): string =>
  `radial-gradient(circle at 20% 20%, ${swatch(preset.primary)}, transparent 70%), radial-gradient(circle at 90% 100%, ${swatch(preset.secondary)}, transparent 70%), ${swatch(preset.primary, 950)}`

// Live preview tiles, driven by local state rather than OSC.
const previewOn = ref(true)
const previewValue = ref(64)
const previewOption = ref(1)
const previewOptions = computed(() => ['Happy', 'Blush', 'Smug', 'Sad'].map((name, index) => ({ key: name, name, selected: index === previewOption.value })))
</script>

<template>
  <FormSection
    title="Theme"
    description="Your theme also shows on your share page, so viewers see the same colors."
  >
    <div class="grid grid-cols-[repeat(auto-fill,minmax(9.5rem,1fr))] gap-2.5">
      <button
        v-for="preset in PRESETS"
        :key="preset.name"
        type="button"
        class="cursor-pointer overflow-hidden rounded-field border text-left transition-colors"
        :class="primary === preset.primary && secondary === preset.secondary ? 'border-highlighted shadow-[inset_0_0_0_1px_var(--ui-text-highlighted)]' : 'border-default hover:border-accented'"
        :aria-pressed="primary === preset.primary && secondary === preset.secondary"
        @click="applyPreset(preset)"
      >
        <span
          class="block h-16"
          :style="{ background: presetBackground(preset) }"
        />
        <span class="flex items-center justify-between gap-1.5 bg-(--aurora-glass) px-3 py-2.5 text-sm font-semibold text-highlighted">
          {{ preset.name }}
          <small class="truncate text-[11.5px] font-normal text-muted">{{ preset.primary }} · {{ preset.secondary }}</small>
        </span>
      </button>
    </div>
  </FormSection>

  <FormSection
    v-for="target in (['primary', 'secondary'] as const)"
    :key="target"
    :title="target === 'primary' ? 'Primary color' : 'Secondary color'"
    :description="target === 'primary' ? 'Buttons, the selected page, and anything switched on.' : 'Values such as sliders and selected options, and the second background glow.'"
  >
    <div class="flex flex-wrap gap-2.5">
      <button
        v-for="color in COLOR_FAMILIES"
        :key="color"
        type="button"
        class="grid size-11.5 cursor-pointer place-items-center rounded-full text-white shadow-[inset_0_0_0_1px_rgb(255_255_255/0.15)] transition-shadow"
        :class="{ 'shadow-[0_0_0_3px_var(--ui-bg),0_0_0_6px_var(--ui-text-highlighted)]': (target === 'primary' ? primary : secondary) === color }"
        :style="{ backgroundColor: swatch(color) }"
        :title="color"
        :aria-label="`${target} color ${color}`"
        :aria-pressed="(target === 'primary' ? primary : secondary) === color"
        @click="setTheme(target, color)"
      >
        <UIcon
          v-if="(target === 'primary' ? primary : secondary) === color"
          name="i-lucide-check"
          class="size-5"
        />
      </button>
    </div>
  </FormSection>

  <FormSection title="Preview">
    <div
      class="tile-grid"
      :data-density="density === 'l' ? 'm' : density"
    >
      <ControlBooleanBase
        v-model="previewOn"
        title="Hoodie"
        icon="i-lucide-shirt"
      />
      <ControlSliderBase
        v-model="previewValue"
        title="Tail wag"
      />
      <div data-span="m">
        <ControlBase
          title="Face"
          kind="value"
          fill
        >
          <ControlOptions
            :options="previewOptions"
            @select="previewOption = $event"
          />
        </ControlBase>
      </div>
    </div>
  </FormSection>

  <FormSection
    title="Tile size"
    description="Pick what fits your overlay window. You can also change it from the Controls page."
  >
    <SegmentedControl
      v-model="density"
      class="w-fit"
      aria-label="Tile size"
      :items="[
        { value: 's', label: 'Small' },
        { value: 'm', label: 'Medium' },
        { value: 'l', label: 'Large' }
      ]"
    />
  </FormSection>
</template>
