<script setup lang="ts">
import type { DropdownMenuItem } from '@nuxt/ui'
import { isControlOn, VALUE_CONTROL_TYPES } from '@renderer/utils/controlState'
import type { ControlGroup } from '@vrc-osc-toolkit/shared-ui'
import { useOscMessages } from '@vrc-osc-toolkit/shared-ui'
import { computed } from 'vue'

// One group in the Controls page's sidebar: name, control count, how many are on, and a row of
// state dots (one per control, capped so a big group can't push the name away). `menu` is the
// group's actions, behind a "⋯" button on the selected group - what VR pointers use, since they
// have no right-click. The right-click menu lives on the Controls page's list (see ControlsPage.vue
// for why it can't wrap each row). `data-group-id` is how that menu finds the row.
const MAX_DOTS = 10

const props = defineProps<{
  group: ControlGroup
  active?: boolean
  editing?: boolean
  selected?: boolean
  // Replaces the count line while searching.
  matches?: number
  menu?: DropdownMenuItem[][]
}>()

const emit = defineEmits<{ (e: 'select'): void; (e: 'toggle-select'): void }>()

const { get } = useOscMessages()

const states = computed(() => props.group.controls.map((control) => ({ id: control.id, on: isControlOn(control, get), value: VALUE_CONTROL_TYPES.has(control.type) })))

const onCount = computed(() => states.value.filter((state) => state.on).length)

const subtitle = computed(() => {
  if (props.matches !== undefined) return `${props.matches} ${props.matches === 1 ? 'match' : 'matches'}`

  const count = `${props.group.controls.length} ${props.group.controls.length === 1 ? 'control' : 'controls'}`

  return [count, onCount.value ? `${onCount.value} on` : null, props.group.hidden ? 'hidden' : null].filter(Boolean).join(' · ')
})
</script>

<template>
  <div
    class="grid min-h-15 w-full cursor-pointer grid-cols-[auto_minmax(0,1fr)_auto] items-center gap-x-2.5 gap-y-0.5 rounded-2xl border px-2.5 py-2 text-left transition-colors"
    :class="[active ? 'border-primary/50 bg-primary/14' : 'border-transparent hover:bg-(--aurora-glass)', { 'opacity-55': group.hidden && !active }]"
    role="button"
    tabindex="0"
    :aria-current="active ? 'true' : undefined"
    :title="group.name"
    :data-group-id="group.id"
    @click="emit('select')"
    @keydown.enter.space.prevent="emit('select')"
  >
    <button
      v-if="editing"
      type="button"
      class="row-span-2 grid size-7 cursor-pointer place-items-center rounded-lg border-2 transition-colors"
      :class="selected ? 'border-primary bg-primary text-inverted' : 'border-(--ui-border-accented) text-transparent'"
      :aria-pressed="selected"
      :aria-label="`Select ${group.name}`"
      @click.stop="emit('toggle-select')"
    >
      <UIcon
        name="i-lucide-check"
        class="size-4"
      />
    </button>
    <span
      v-else
      class="row-span-2 grid size-8.5 place-items-center rounded-xl bg-(--aurora-well)"
      :class="active ? 'text-primary' : 'text-muted'"
    >
      <UIcon
        :name="group.hidden ? 'i-lucide-eye-off' : 'i-lucide-folder'"
        class="size-4.5"
      />
    </span>

    <b class="truncate text-[14.5px] font-semibold text-highlighted">{{ group.name }}</b>
    <small class="col-start-2 truncate text-xs text-muted">{{ subtitle }}</small>

    <span
      v-if="editing"
      class="group-handle col-start-3 row-span-2 row-start-1 grid size-9 cursor-grab place-items-center rounded-xl bg-(--aurora-well) text-muted active:cursor-grabbing"
      title="Drag to reorder"
      @click.stop
    >
      <UIcon
        name="i-lucide-grip-vertical"
        class="size-4"
      />
    </span>
    <UDropdownMenu
      v-else-if="active && menu?.length"
      :items="menu"
      :content="{ align: 'start', side: 'right' }"
    >
      <UButton
        icon="i-lucide-ellipsis"
        color="neutral"
        variant="ghost"
        size="md"
        class="col-start-3 row-span-2 row-start-1"
        :aria-label="`${group.name} actions`"
        @click.stop
      />
    </UDropdownMenu>
    <span
      v-else
      class="col-start-3 row-span-2 row-start-1 flex max-w-16 flex-wrap items-center justify-end gap-0.75"
      aria-hidden="true"
    >
      <i
        v-for="state in states.slice(0, MAX_DOTS)"
        :key="state.id"
        class="size-1.5 rounded-full"
        :class="state.on ? 'bg-primary shadow-[0_0_4px_var(--ui-primary)]' : state.value ? 'bg-secondary' : 'bg-(--ui-text-dimmed)'"
      />
      <span
        v-if="states.length > MAX_DOTS"
        class="font-mono text-[10px] text-muted"
      >+{{ states.length - MAX_DOTS }}</span>
    </span>
  </div>
</template>
