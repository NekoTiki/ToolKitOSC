<script setup lang="ts">
import { computed } from 'vue'

// The leash seen from above: the needle points where you're pulled (forward is up) and grows with
// the stretch. Inside the inner ring you stand, past it you walk, past the outer ring you run.
const props = defineProps<{
  forward: number
  right: number
  stretch: number
  walk: number
  run: number
  held: boolean
}>()

const CENTER = 110
const RADIUS = 100

const needle = computed(() => {
  const length = Math.hypot(props.right, props.forward)

  if (!props.held || length === 0 || props.stretch === 0) return null

  const reach = Math.min(props.stretch, 1) * RADIUS

  return { x: CENTER + (props.right / length) * reach, y: CENTER - (props.forward / length) * reach }
})
</script>

<template>
  <svg
    viewBox="0 0 220 220"
    class="block h-auto w-full max-w-56"
    role="img"
    :aria-label="needle ? `Pulled ${forward >= 0 ? 'forward' : 'back'} and ${right >= 0 ? 'right' : 'left'}` : 'Not pulled'"
  >
    <circle
      :cx="CENTER"
      :cy="CENTER"
      :r="RADIUS"
      class="fill-(--aurora-well) stroke-(--ui-border)"
      stroke-width="2"
    />
    <line
      :x1="CENTER"
      y1="14"
      :x2="CENTER"
      y2="206"
      class="stroke-(--ui-border)"
      stroke-dasharray="3 4"
    />
    <line
      x1="14"
      :y1="CENTER"
      x2="206"
      :y2="CENTER"
      class="stroke-(--ui-border)"
      stroke-dasharray="3 4"
    />
    <circle
      :cx="CENTER"
      :cy="CENTER"
      :r="walk * RADIUS"
      fill="none"
      class="stroke-secondary"
      stroke-width="1.5"
      opacity=".6"
    />
    <circle
      :cx="CENTER"
      :cy="CENTER"
      :r="run * RADIUS"
      fill="none"
      class="stroke-warning"
      stroke-width="1.5"
      opacity=".7"
    />
    <text
      :x="CENTER"
      y="30"
      text-anchor="middle"
      class="fill-(--ui-text-muted) text-[11px] font-semibold"
    >Front</text>
    <g v-if="needle">
      <line
        :x1="CENTER"
        :y1="CENTER"
        :x2="needle.x"
        :y2="needle.y"
        class="stroke-primary"
        stroke-width="4"
        stroke-linecap="round"
      />
      <circle
        :cx="needle.x"
        :cy="needle.y"
        r="6"
        class="fill-primary"
      />
    </g>
    <circle
      :cx="CENTER"
      :cy="CENTER"
      r="5"
      class="fill-(--ui-text-highlighted)"
    />
  </svg>
</template>
