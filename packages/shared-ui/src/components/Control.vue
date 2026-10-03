<script setup lang="ts">
import { computed, provide } from 'vue'

import type { CommandWithoutIds, ControlLimits, ControlType, LastUser } from '../types/controls'
import ControlBoolean from './ControlBoolean.vue'
import ControlBooleanEnum from './ControlBooleanEnum.vue'
import ControlBooleanGroup from './ControlBooleanGroup.vue'
import ControlEnum from './ControlEnum.vue'
import ControlIntifacePattern from './ControlIntifacePattern.vue'
import ControlIntifaceToy from './ControlIntifaceToy.vue'
import ControlOpenShock from './ControlOpenShock.vue'
import ControlPreset from './ControlPreset.vue'
import ControlSlider from './ControlSlider.vue'
import ControlStepEnum from './ControlStepEnum.vue'
import type { TileBadge } from './tileBadges'
import { TILE_BADGES } from './tileBadges'

type Props = {
  lastUser?: LastUser
  control: ControlType
  // Edit mode (desktop client): large Edit / Activity / Delete buttons over the tile, and a drag
  // handle. Replaces the old right-click menu, which VR pointers and touch can't open.
  edit?: boolean
  offline?: boolean
  // Blocking states (viewer side): the tile can't be used at all.
  locked?: boolean
  unavailable?: boolean
  // The active profile's limits (viewer side): blocked options can't be picked, sliders stop at the
  // range.
  limits?: ControlLimits
  // Informational states (host side): the host can still use the tile, but viewers can't.
  lockedIndicator?: boolean
  unavailableIndicator?: boolean
  limitedIndicator?: boolean
}

const props = defineProps<Props>()
defineEmits<{
  (e: 'edit', id: string): void
  (e: 'logs', id: string): void
  (e: 'delete', id: string): void
  (e: 'command', command: CommandWithoutIds): void
}>()

// Every control type that can go unavailable has its own reason, shown on the tile itself.
const UNAVAILABLE_REASON: Partial<Record<ControlType['type'], string>> = {
  'open-shock-shocker': "OpenShock isn't set up",
  'intiface-toy': 'Intiface or its toys are offline',
  'intiface-pattern': 'Intiface or its toys are offline',
  preset: 'This preset no longer exists'
}

const unavailableReason = computed(() => UNAVAILABLE_REASON[props.control.type] ?? 'Not available right now')

provide(
  TILE_BADGES,
  computed<TileBadge[]>(() => [
    ...(props.lockedIndicator ? [{ key: 'locked', icon: 'i-lucide-lock', label: 'Locked for viewers by the active profile', tone: 'warning' as const }] : []),
    ...(props.limitedIndicator && !props.lockedIndicator
      ? [{ key: 'limited', icon: 'i-lucide-sliders-horizontal', label: 'Limited for viewers by the active profile', tone: 'info' as const }]
      : []),
    ...(props.unavailableIndicator && !props.lockedIndicator
      ? [{ key: 'unavailable', icon: 'i-lucide-shield-off', label: `${unavailableReason.value}, so viewers can't use it`, tone: 'warning' as const }]
      : []),
    ...(props.lastUser ? [{ key: 'last-user', avatar: props.lastUser.avatar, label: `Last used by ${props.lastUser.displayName}` }] : [])
  ])
)

// Locked/unavailable/offline tiles show a hatched scrim with the reason at the bottom, instead of
// the old full black cover - the tile's name and state stay readable underneath.
const blocker = computed<{ icon: string; label: string } | null>(() => {
  if (props.offline) return { icon: 'i-lucide-wifi-off', label: 'Host offline' }
  if (props.locked) return { icon: 'i-lucide-lock', label: 'Locked' }
  if (props.unavailable) return { icon: 'i-lucide-shield-off', label: unavailableReason.value }

  return null
})
</script>

<template>
  <div
    class="relative isolate h-full w-full"
    :class="{ grayscale: offline }"
  >
    <control-boolean
      v-if="control.type === 'boolean'"
      :icon="control.icon"
      :title="control.name"
      :reverse="control.reverse"
      :address="control.inputAddress"
      @update:value="$emit('command', { type: 'boolean', value: $event })"
    />
    <control-boolean-group
      v-else-if="control.type === 'boolean-group'"
      :icon="control.icon"
      :title="control.name"
      :inputs="control.inputs.map((c) => ({ address: c.inputAddress }))"
      @update:value="$emit('command', { type: 'boolean-group', value: $event })"
    />
    <control-boolean-enum
      v-else-if="control.type === 'boolean-enum'"
      :icon="control.icon"
      :title="control.name"
      :items="control.inputs"
      :disabled-options="limits?.disabledOptions"
      @update:value="$emit('command', { type: 'boolean-enum', value: $event })"
    />
    <control-enum
      v-else-if="control.type === 'enum'"
      :icon="control.icon"
      :title="control.name"
      :address="control.inputAddress"
      :items="control.options"
      :disabled-options="limits?.disabledOptions"
      @update:value="$emit('command', { type: 'enum', value: $event })"
    />
    <control-slider
      v-else-if="control.type === 'slider'"
      :title="control.name"
      :icon="control.icon"
      :address="control.inputAddress"
      :limits="limits"
      @update:value="$emit('command', { type: 'slider', value: $event })"
    />
    <control-step-enum
      v-else-if="control.type === 'step-enum'"
      :icon="control.icon"
      :title="control.name"
      :address="control.inputAddress"
      :items="control.options"
      @update:value="$emit('command', { type: 'step-enum', value: $event })"
    />
    <control-open-shock
      v-else-if="control.type === 'open-shock-shocker'"
      :control-id="control.id"
      :title="control.name"
      :mode="control.mode"
      :intensity="control.intensity"
      :duration="control.duration"
      @run="$emit('command', { type: 'open-shock-shocker' })"
    />
    <control-intiface-toy
      v-else-if="control.type === 'intiface-toy'"
      :control-id="control.id"
      :title="control.name"
      :icon="control.icon"
      :limits="limits"
      @update:value="$emit('command', { type: 'intiface-toy', value: $event })"
    />
    <control-intiface-pattern
      v-else-if="control.type === 'intiface-pattern'"
      :control-id="control.id"
      :title="control.name"
      :icon="control.icon"
      :patterns="control.allowedPatterns"
      :disabled-options="limits?.disabledOptions"
      @update:value="$emit('command', { type: 'intiface-pattern', value: $event })"
    />
    <control-preset
      v-else-if="control.type === 'preset'"
      :title="control.name"
      :icon="control.icon"
      @run="$emit('command', { type: 'preset' })"
    />

    <div
      v-if="blocker && !edit"
      class="absolute inset-0 z-20 flex cursor-not-allowed items-end rounded-panel bg-black/45 bg-[repeating-linear-gradient(135deg,transparent_0_10px,rgb(255_255_255/0.05)_10px_20px)] p-(--tile-pad)"
      @click.stop
    >
      <span class="flex min-w-0 items-center gap-1.5 rounded-full bg-black/60 px-2.5 py-1 text-xs font-medium text-warning">
        <UIcon
          :name="blocker.icon"
          class="size-3.5 shrink-0"
        />
        <span class="truncate">{{ blocker.label }}</span>
      </span>
    </div>

    <!-- Edit mode: labeled, icon-over-text buttons big enough for a VR pointer. `.handle` is the
    only element vue-draggable-plus grabs to reorder, so clicks on the other buttons never start a
    drag. -->
    <div
      v-if="edit"
      class="absolute inset-0 z-20 flex flex-col items-center justify-center gap-2 rounded-panel bg-(--ui-bg)/75 p-2 ring-2 ring-primary/60"
    >
      <span
        class="handle flex cursor-grab items-center gap-1 rounded-full bg-primary py-1 pr-3 pl-2 text-xs font-semibold text-inverted select-none active:cursor-grabbing"
        title="Drag to move"
      >
        <UIcon
          name="i-lucide-grip-vertical"
          class="size-4"
        />
        Move
      </span>
      <div class="flex gap-2">
        <button
          v-for="action in [
            { key: 'edit', icon: 'i-lucide-pen', label: 'Edit' },
            { key: 'logs', icon: 'i-lucide-history', label: 'Activity' },
            { key: 'delete', icon: 'i-lucide-trash-2', label: 'Delete' }
          ] as const"
          :key="action.key"
          type="button"
          class="flex size-[clamp(2.75rem,calc(var(--tile-cell)*0.3),3.5rem)] cursor-pointer flex-col items-center justify-center gap-0.5 rounded-field border border-default bg-elevated text-[10px] transition-colors hover:bg-accented"
          :class="action.key === 'delete' ? 'text-error' : 'text-highlighted'"
          :aria-label="action.label"
          @click.stop="action.key === 'edit' ? $emit('edit', control.id) : action.key === 'logs' ? $emit('logs', control.id) : $emit('delete', control.id)"
        >
          <UIcon
            :name="action.icon"
            class="size-5"
          />
          {{ action.label }}
        </button>
      </div>
    </div>
  </div>
</template>
