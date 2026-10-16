<script setup lang="ts">
import type { StatusTone } from '@renderer/components/layout/StatusChip.vue'
import type { Addon, AddonState } from '@renderer/composables/useAddons'
import { ADDON_SETUP, useAddons } from '@renderer/composables/useAddons'
import { computed, ref } from 'vue'
import { useRouter } from 'vue-router'

// The top bar's Add-ons chip: one dot per add-on that's on, the chip itself in the worst of their
// tones, and a popover (opened by a tap, never hover) listing each one's status. A row opens that
// add-on's settings.
const { addons } = useAddons()
const router = useRouter()

const open = ref(false)

const TONE: Record<AddonState, StatusTone> = { ok: 'success', busy: 'warning', warn: 'warning', error: 'error', off: 'muted' }
const RANK: StatusTone[] = ['error', 'warning', 'success', 'muted']

const DOT: Record<StatusTone, string> = {
  success: 'bg-success shadow-[0_0_6px_var(--ui-success)]',
  warning: 'bg-warning shadow-[0_0_6px_var(--ui-warning)]',
  error: 'bg-error',
  muted: 'bg-(--ui-text-dimmed)'
}

const enabled = computed(() => addons.value.filter((addon) => addon.enabled))

const tone = computed<StatusTone>(() => RANK.find((t) => enabled.value.some((addon) => TONE[addon.status.state] === t)) ?? 'muted')

const summary = computed(() => {
  const needYou = enabled.value.filter((addon) => TONE[addon.status.state] !== 'success').length

  return needYou ? `${needYou} of ${enabled.value.length} add-ons need a look` : `${enabled.value.length} add-ons, all fine`
})

const go = (to: string): void => {
  open.value = false
  void router.push(to)
}

const openAddon = (addon: Addon): void => go(ADDON_SETUP[addon.id] ?? '/settings/addons')
</script>

<template>
  <UPopover
    v-if="enabled.length"
    v-model:open="open"
    :content="{ align: 'end' }"
  >
    <StatusChip
      icon="i-lucide-puzzle"
      label="Add-ons"
      :tone="tone"
      :title="summary"
    >
      <span
        class="flex items-center gap-1"
        aria-hidden="true"
      >
        <span
          v-for="addon in enabled"
          :key="addon.id"
          class="size-2 rounded-full"
          :class="[DOT[TONE[addon.status.state]], addon.status.state === 'busy' && 'animate-pulse']"
        />
      </span>
    </StatusChip>

    <template #content>
      <div class="grid w-80 gap-1 p-2">
        <button
          v-for="addon in enabled"
          :key="addon.id"
          type="button"
          class="grid min-h-13 cursor-pointer grid-cols-[auto_minmax(0,1fr)_auto] items-center gap-x-3 rounded-2xl px-3 py-2 text-left transition-colors hover:bg-elevated"
          @click="openAddon(addon)"
        >
          <span
            class="size-2.5 rounded-full"
            :class="[DOT[TONE[addon.status.state]], addon.status.state === 'busy' && 'animate-pulse']"
          />
          <span class="grid min-w-0">
            <b class="truncate text-sm font-semibold text-highlighted">{{ addon.name }}</b>
            <small
              class="truncate text-xs"
              :class="addon.status.state === 'error' ? 'text-error' : 'text-muted'"
            >{{ addon.status.text }}</small>
          </span>
          <UIcon
            name="i-lucide-chevron-right"
            class="size-4 text-muted"
          />
        </button>
        <UButton
          icon="i-lucide-settings-2"
          color="neutral"
          variant="subtle"
          block
          class="mt-1 rounded-2xl"
          @click="go('/settings/addons')"
        >
          Manage add-ons
        </UButton>
      </div>
    </template>
  </UPopover>
</template>
