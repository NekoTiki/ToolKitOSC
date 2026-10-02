<script setup lang="ts">
// A full-page state on the share page (connecting, disconnected, sign-in, banned, nothing
// shared): one centered card with an icon, a title, an explanation and large actions - replacing
// the old modals, which were easy to dismiss by accident and awkward to use from VR.
defineProps<{
  icon: string
  title: string
  description?: string
  tone?: 'default' | 'error'
  spin?: boolean
}>()

defineSlots<{ default?(): unknown }>()
</script>

<template>
  <div class="grid h-full place-items-center p-5">
    <div class="grid w-full max-w-md justify-items-center gap-3.5 rounded-panel glass-strong p-7 text-center">
      <UIcon
        :name="icon"
        class="size-12"
        :class="[tone === 'error' ? 'text-error' : 'text-muted', { 'animate-spin': spin }]"
      />
      <h1 class="text-[26px] font-semibold text-highlighted">
        {{ title }}
      </h1>
      <p
        v-if="description"
        class="text-[14.5px] leading-relaxed text-muted"
      >
        {{ description }}
      </p>
      <div
        v-if="$slots.default"
        class="grid w-full gap-2.5"
      >
        <slot />
      </div>
    </div>
  </div>
</template>
