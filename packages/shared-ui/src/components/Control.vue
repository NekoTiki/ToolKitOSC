<script setup lang="ts">
import { computed } from 'vue'

import type { CommandWithoutIds, ControlType, LastUser } from '../types/controls'
import ControlBoolean from './ControlBoolean.vue'
import ControlBooleanEnum from './ControlBooleanEnum.vue'
import ControlBooleanGroup from './ControlBooleanGroup.vue'
import ControlEnum from './ControlEnum.vue'
import ControlIntifacePattern from './ControlIntifacePattern.vue'
import ControlIntifaceToy from './ControlIntifaceToy.vue'
import ControlOpenShock from './ControlOpenShock.vue'
import ControlPreset from './ControlPreset.vue'
import ControlSlider from './ControlSlider.vue'

type Props = {
  lastUser?: LastUser
  control: ControlType
  edit?: boolean
  offline?: boolean
  locked?: boolean
  lockedIndicator?: boolean
  unavailable?: boolean
  unavailableIndicator?: boolean
}

const props = defineProps<Props>()
defineEmits<{
  (e: 'edit', id: string): void
  (e: 'delete', id: string): void
  (e: 'command', command: CommandWithoutIds): void
}>()

// The unavailable-indicator tooltip used to always say "OpenShock isn't configured", which was
// wrong for any other control type that can go unavailable (currently also Intiface, which can be
// unreachable for two different reasons of its own).
const UNAVAILABLE_REASON: Partial<Record<ControlType['type'], string>> = {
  'open-shock-shocker': "OpenShock isn't configured",
  'intiface-toy': "Intiface isn't connected, or every toy this control targets is offline",
  'intiface-pattern': "Intiface isn't connected, or every toy this control targets is offline",
  preset: "This preset no longer exists for the currently loaded avatar"
}

const unavailableReason = computed(
  () => UNAVAILABLE_REASON[props.control.type] ?? "Something this control depends on isn't available"
)
</script>

<template>
  <div class="relative isolate">
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
      @update:value="$emit('command', { type: 'boolean-enum', value: $event })"
    />
    <control-enum
      v-else-if="control.type === 'enum'"
      :icon="control.icon"
      :title="control.name"
      :address="control.inputAddress"
      :items="control.options"
      @update:value="$emit('command', { type: 'enum', value: $event })"
    />
    <control-slider
      v-else-if="control.type === 'slider'"
      :title="control.name"
      :icon="control.icon"
      :address="control.inputAddress"
      @update:value="$emit('command', { type: 'slider', value: $event })"
    />
    <!-- Same picker UI as 'enum' - StepEnumControl is structurally identical to EnumControl (an
    address + named integer options), just the AI-suggestible variant of it (see server's ai/
    normalize.ts). This case was missing entirely, so an AI-generated step-enum control silently
    rendered nothing at all - the same class of bug migrateControlGroups (useControls.ts) already
    has to work around for the old 'intiface-vibrator' rename. -->
    <control-enum
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
      @update:value="$emit('command', { type: 'intiface-toy', value: $event })"
    />
    <control-intiface-pattern
      v-else-if="control.type === 'intiface-pattern'"
      :control-id="control.id"
      :title="control.name"
      :icon="control.icon"
      :patterns="control.allowedPatterns"
      @update:value="$emit('command', { type: 'intiface-pattern', value: $event })"
    />
    <control-preset
      v-else-if="control.type === 'preset'"
      :title="control.name"
      :icon="control.icon"
      @run="$emit('command', { type: 'preset' })"
    />
    <UPopover
      v-if="lastUser"
      class="absolute top-4 right-4 z-20 flex items-center"
      :content="{ align: 'center', side: 'left', sideOffset: 8 }"
    >
      <UAvatar
        :src="lastUser.avatar"
        size="xs"
      />

      <template #content>
        <slot
          name="last-user"
          :last-user="lastUser"
        >
          {{ lastUser }}
        </slot>
      </template>
    </UPopover>
    <div
      v-if="edit"
      class="absolute inset-0 z-20 flex h-full w-full flex-col items-center justify-between gap-2 rounded-lg bg-primary/15 p-2 ring-2 ring-primary"
    >
      <!-- A primary-tinted scrim + ring, not a generic dark one - this reuses the same
      "bg-primary/10 ring-primary" language ControlBase.vue already uses for its own `active`
      state, so "editing" reads as its own distinct mode rather than a disabled/unavailable look
      (those still use a flat black scrim - see the offline/locked/unavailable divs below). -->
      <!-- The `.handle` class used to sit on UTooltip itself, forwarded onto its slotted child via
      Reka UI's `as-child` prop-merging - fragile, since that merge target is an asynchronously-
      rendered UIcon rather than an element guaranteed to exist as soon as this renders.
      vue-draggable-plus's `handle: '.handle'` selector needs a real, always-present node to grab,
      so it's now a plain span we render ourselves instead. -->
      <UTooltip>
        <span
          class="handle bg-primary text-inverted flex cursor-grab items-center gap-1 rounded-full py-1 pr-3 pl-2 text-xs font-bold shadow-md transition-transform select-none hover:scale-105 active:cursor-grabbing active:scale-95"
        >
          <UIcon
            name="i-lucide-grip-vertical"
            class="size-4"
          />
          Drag
        </span>

        <template #content>
          Drag to Reorder
        </template>
      </UTooltip>

      <!-- Labeled, not icon-only, and stretched across the full width - a bigger, harder-to-
      mis-tap target than the small icon buttons this replaced, and the label removes any doubt
      about which button does what while a whole grid of controls is in edit mode at once. A plain
      flex row with a gap, not UButtonGroup - that component joins its buttons into one seamless
      segmented control (shared edges, no gap by design), which isn't what's wanted between two
      unrelated actions like these. `variant="subtle"` for Edit instead of `"solid"`: solid+neutral
      inverts to a stark white pill in dark mode, too bright against this already-light
      `bg-primary/15` scrim - Delete keeps `"solid"` since its error red doesn't have that
      light/dark inversion problem. -->
      <div class="flex w-full gap-2">
        <UButton
          label="Edit"
          icon="i-lucide-pen-line"
          color="neutral"
          variant="subtle"
          block
          class="grow"
          @click="$emit('edit', control.id)"
        />
        <UButton
          label="Delete"
          icon="i-lucide-trash-2"
          color="error"
          variant="solid"
          block
          class="grow"
          @click="$emit('delete', control.id)"
        />
      </div>
    </div>
    <div
      v-if="offline"
      class="absolute inset-0 z-20 flex h-full w-full cursor-pointer items-center justify-center rounded-lg bg-black/70"
    >
      <div class="flex flex-wrap items-center justify-center gap-4 text-white">
        <UIcon
          name="i-lucide-wifi-off"
          class="size-16"
        />
        <span class="w-full text-center">Host Offline</span>
      </div>
    </div>
    <div
      v-if="locked && !offline"
      class="absolute inset-0 z-20 flex h-full w-full cursor-pointer items-center justify-center rounded-lg bg-black/50 dark:bg-black/70"
    >
      <div class="flex flex-wrap items-center justify-center gap-4 text-white">
        <UIcon
          name="lucide:lock"
          class="size-16"
        />
        <span class="w-full text-center">Unavailable</span>
      </div>
    </div>
    <div
      v-if="unavailable && !offline && !locked"
      class="absolute inset-0 z-20 flex h-full w-full cursor-pointer items-center justify-center rounded-lg bg-black/50 dark:bg-black/70"
    >
      <div class="flex flex-wrap items-center justify-center gap-4 text-white">
        <UIcon
          name="lucide:shield-off"
          class="size-16"
        />
        <span class="w-full text-center">Unavailable</span>
      </div>
    </div>
    <!-- Bottom-left, not top-left: some control types (OpenShock) render their own tooltip icon
    (e.g. shock/vibrate mode) at top-3 left-3 in their own component, and the edit-mode drag
    handle above also claims that corner - either would sit directly on top of these otherwise. -->
    <UTooltip
      v-if="lockedIndicator"
      class="absolute bottom-3 left-3 z-20 cursor-grab rounded-lg"
    >
      <UIcon
        name="lucide:lock"
        class="size-6"
      />

      <template #content>
        This control is seen as locked
      </template>
    </UTooltip>
    <UTooltip
      v-if="unavailableIndicator && !lockedIndicator"
      class="absolute bottom-3 left-3 z-20 cursor-grab rounded-lg"
    >
      <UIcon
        name="lucide:shield-off"
        class="size-6"
      />

      <template #content>
        {{ unavailableReason }} - this control is unavailable to everyone
      </template>
    </UTooltip>
  </div>
</template>

<style scoped></style>
