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
  'intiface-pattern': "Intiface isn't connected, or every toy this control targets is offline"
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
      class="absolute inset-0 z-20 flex h-full w-full cursor-pointer items-center justify-center rounded-lg bg-black/70"
    >
      <!-- The `.handle` class used to sit on UTooltip itself, forwarded onto its slotted child via
      Reka UI's `as-child` prop-merging - fragile, since that merge target is an asynchronously-
      rendered UIcon rather than an element guaranteed to exist as soon as this renders.
      vue-draggable-plus's `handle: '.handle'` selector needs a real, always-present node to grab,
      so it's now a plain span we render ourselves instead. Same grip icon/style as the group
      drag handle (see ControlGroup.vue's `.group-handle`), just with explicit white tones instead
      of the theme-aware `text-muted`/`text-default` tokens that handle uses - this one always sits
      on the same hardcoded dark overlay, in both light and dark mode. -->
      <UTooltip>
        <span
          class="handle absolute top-2 left-2 flex cursor-grab items-center justify-center rounded-lg p-1.5 text-white/70 transition-colors hover:bg-white/10 hover:text-white active:cursor-grabbing"
        >
          <UIcon
            name="i-lucide-grip-vertical"
            class="size-5"
          />
        </span>

        <template #content>
          Drag to Reorder
        </template>
      </UTooltip>

      <div class="absolute top-2 right-2 flex gap-2 rounded-lg">
        <UButton
          icon="fa7-solid:trash"
          color="error"
          variant="subtle"
          @click="$emit('delete', control.id)"
        />
        <UButton
          icon="fa7-solid:pen"
          variant="subtle"
          @click="$emit('edit', control.id)"
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
