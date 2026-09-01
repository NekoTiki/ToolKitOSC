<script setup lang="ts">
import type { CommandWithoutIds, ControlType, LastUser } from '../types/controls'
import ControlBoolean from './ControlBoolean.vue'
import ControlBooleanEnum from './ControlBooleanEnum.vue'
import ControlBooleanGroup from './ControlBooleanGroup.vue'
import ControlEnum from './ControlEnum.vue'
import ControlOpenShock from './ControlOpenShock.vue'
import ControlSlider from './ControlSlider.vue'

type Props = {
  lastUser?: LastUser
  control: ControlType
  edit?: boolean
  offline?: boolean
  locked?: boolean
  lockedIndicator?: boolean
}

defineProps<Props>()
defineEmits<{
  (e: 'edit', id: string): void
  (e: 'delete', id: string): void
  (e: 'command', command: CommandWithoutIds): void
}>()
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

      <UTooltip class="handle absolute top-3 left-3 cursor-grab rounded-lg">
        <UIcon
          name="iconamoon:menu-burger-horizontal-light"
          class="size-6"
        />

        <template #content>
          Drag to Reorder
        </template>
      </UTooltip>
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
    <UTooltip
      v-if="lockedIndicator"
      class="absolute top-3 left-3 z-20 cursor-grab rounded-lg"
    >
      <UIcon
        name="lucide:lock"
        class="size-6"
      />

      <template #content>
        This control is seen as locked
      </template>
    </UTooltip>
  </div>
</template>

<style scoped></style>
