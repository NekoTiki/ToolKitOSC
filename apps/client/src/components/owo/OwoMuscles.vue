<script setup lang="ts">
import { OWO_MUSCLES } from '@renderer/composables/useOwo'
import type { OwoMuscle, OwoSide } from '@renderer/lib/tauri-bridge'
import { computed } from 'vue'

// One side of the OWO Skin as it plays now, laid out as you'd see it: facing the wearer for the
// front (their right on your left), from behind for the back.
const props = defineProps<{
  side: OwoSide
  // 0-100 per muscle, missing while still.
  levels?: Partial<Record<OwoMuscle, number>>
}>()

const LAYOUT: Record<OwoSide, OwoMuscle[]> = {
  front: ['PectoralR', 'PectoralL', 'AbdominalR', 'AbdominalL', 'ArmR', 'ArmL'],
  back: ['DorsalL', 'DorsalR', 'LumbarL', 'LumbarR']
}

const muscles = computed(() => LAYOUT[props.side])
const level = (muscle: OwoMuscle): number => (props.levels?.[muscle] ?? 0) / 100
</script>

<template>
  <div
    class="grid w-fit grid-cols-[repeat(2,4.5rem)] gap-1.5 rounded-t-[1.6rem] rounded-b-xl bg-(--aurora-well) p-3"
    role="img"
    :aria-label="`${side === 'front' ? 'Front' : 'Back'}: ${muscles.filter((m) => level(m) > 0).length} muscles playing`"
  >
    <span
      v-for="muscle in muscles"
      :key="muscle"
      class="grid h-9 place-items-center rounded-xl text-[11px] text-muted shadow-[inset_0_0_0_1px_var(--ui-border)] transition-[background-color,box-shadow] duration-100"
      :style="
        level(muscle) > 0
          ? {
            backgroundColor: `color-mix(in srgb, var(--ui-warning) ${Math.round(35 + level(muscle) * 65)}%, transparent)`,
            boxShadow: `0 0 ${Math.round(level(muscle) * 12)}px var(--ui-warning)`,
            color: 'var(--ui-text-highlighted)'
          }
          : undefined
      "
    >{{ OWO_MUSCLES[muscle].short }}</span>
  </div>
</template>
