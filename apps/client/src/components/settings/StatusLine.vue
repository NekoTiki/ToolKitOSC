<script setup lang="ts">
// The small "● Connected" line under a connection setting.
export interface StatusInfo {
  icon: string
  class: string
  label: string
}

defineProps<{ status: StatusInfo }>()
</script>

<template>
  <div class="flex items-center gap-1.5 text-[13px]">
    <UIcon
      :name="status.icon"
      class="size-4"
      :class="status.class"
    />
    <span :class="status.class">{{ status.label }}</span>
  </div>
</template>
