<script setup lang="ts">
// A checkbox in a chip, the editor's multi-pick (shockers, toys, muscles, effects, patterns): the
// whole chip toggles it, a big target for a VR pointer. The default slot replaces the label text;
// `end` holds a button beside it, outside the <label> so pressing it doesn't toggle.
//
// `id` must be unique on the page: several checkboxes inside one UFormField would otherwise all
// inherit its id, and every <label for> would point at the first one.
defineProps<{ id: string; label?: string }>()

const checked = defineModel<boolean>({ required: true })
</script>

<template>
  <div class="flex min-h-12 items-center rounded-2xl border border-default bg-(--aurora-well)">
    <label
      :for="id"
      class="flex min-h-12 min-w-0 flex-1 cursor-pointer items-center gap-3 px-3"
    >
      <UCheckbox
        :id="id"
        :model-value="checked"
        @update:model-value="checked = !!$event"
      />
      <slot :checked="checked">
        <span :class="checked ? 'text-highlighted' : 'text-muted'">{{ label }}</span>
      </slot>
    </label>
    <slot name="end" />
  </div>
</template>
