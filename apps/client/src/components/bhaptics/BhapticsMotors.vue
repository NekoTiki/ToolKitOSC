<script setup lang="ts">
import { BHAPTICS_POSITIONS } from '@renderer/composables/useBhaptics'
import type { BhapticsPosition } from '@renderer/lib/tauri-bridge'
import { computed } from 'vue'

// One position's motors as they play now, in the order Player numbers them: left to right, top to
// bottom, as you'd see them facing the wearer (or from behind, for the back).
const props = defineProps<{
  position: BhapticsPosition
  // 0-100 per motor, or nothing while it's still.
  levels?: number[]
}>()

const layout = computed(() => BHAPTICS_POSITIONS[props.position])
const level = (motor: number): number => (props.levels?.[motor] ?? 0) / 100
</script>

<template>
  <div
    class="grid w-fit gap-1.5 bg-(--aurora-well) p-3"
    :class="position === 'VestFront' || position === 'VestBack' ? 'rounded-t-[1.6rem] rounded-b-xl' : 'rounded-xl'"
    :style="{ gridTemplateColumns: `repeat(${layout.columns}, 1.25rem)` }"
    role="img"
    :aria-label="`${layout.label}: ${levels?.filter((l) => l > 0).length ?? 0} motors playing`"
  >
    <span
      v-for="motor in layout.motors"
      :key="motor"
      class="size-5 rounded-full shadow-[inset_0_0_0_1px_var(--ui-border)] transition-[background-color,box-shadow] duration-100"
      :style="
        level(motor - 1) > 0
          ? {
            backgroundColor: `color-mix(in srgb, var(--ui-warning) ${Math.round(35 + level(motor - 1) * 65)}%, transparent)`,
            boxShadow: `0 0 ${Math.round(level(motor - 1) * 12)}px var(--ui-warning)`
          }
          : undefined
      "
    />
  </div>
</template>
