<script setup lang="ts">
// The shared shell for every confirmation dialog (delete, ban, clear, sign out...), so they all
// look the same: a title (with a warning icon when the action is destructive), a short muted
// message, optional extra fields, and two big side-by-side buttons that are easy to hit from VR.
// Confirmations are the only dialogs left in the app - everything else is a page.
withDefaults(
  defineProps<{
    title: string
    confirmText: string
    cancelText?: string
    confirmIcon?: string
    // Destructive actions get the warning icon and a red confirm button; the rest (unban...) use
    // the primary color.
    danger?: boolean
  }>(),
  { cancelText: 'Cancel', confirmIcon: undefined, danger: true }
)

const emit = defineEmits<{ confirm: []; cancel: []; 'after:leave': [] }>()
const open = defineModel<boolean>('open')

// default: the message. `**bold**`-style call-outs should be <strong>, which is highlighted.
// fields: anything to fill in before confirming (the ban scope and reason).
defineSlots<{ default(): unknown; fields?(): unknown }>()
</script>

<template>
  <!-- Every Modal/Slideover in this app defaults to the same z-30 (set globally in
  vite.config.ts), which is normally fine since they don't overlap - but a confirmation is
  routinely opened from INSIDE something else that's already open. Two equal z-30 overlays would
  fall back to DOM/mount order, so confirmations are deliberately bumped above that shared tier -
  and above the z-40 tooltip/popover/dropdown/select tier too, so they win even if something like
  that is still open. -->
  <UModal
    v-model:open="open"
    :title="title"
    :close="false"
    :ui="{
      overlay: 'z-50 bg-black/50 backdrop-blur-xs',
      content: 'z-50 max-w-120 gap-3.5 divide-y-0 rounded-panel p-6 shadow-[0_30px_70px_-30px_rgb(0_0_0/0.8)]',
      header: 'min-h-0 p-0 sm:px-0',
      title: 'flex items-center gap-2.5 text-[22px] leading-tight',
      body: 'p-0 sm:p-0',
      footer: 'mt-1.5 gap-2.5 p-0 sm:px-0'
    }"
    @after:leave="emit('after:leave')"
  >
    <template #title>
      <UIcon
        v-if="danger"
        name="i-lucide-triangle-alert"
        class="size-5.5 shrink-0 text-error"
      />
      <span class="min-w-0 wrap-break-word">{{ title }}</span>
    </template>
    <template #body>
      <div class="grid gap-3.5">
        <div class="text-sm/normal wrap-break-word whitespace-pre-wrap text-muted [&_strong]:font-semibold [&_strong]:text-highlighted">
          <slot />
        </div>
        <slot name="fields" />
      </div>
    </template>
    <template #footer>
      <UButton
        color="neutral"
        variant="subtle"
        size="xl"
        block
        class="h-14 flex-1 text-[15px]"
        @click="emit('cancel')"
      >
        {{ cancelText }}
      </UButton>
      <UButton
        :color="danger ? 'error' : 'primary'"
        size="xl"
        block
        :icon="confirmIcon"
        class="h-14 flex-1 text-[15px]"
        @click="emit('confirm')"
      >
        {{ confirmText }}
      </UButton>
    </template>
  </UModal>
</template>
