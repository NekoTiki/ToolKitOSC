<script setup lang="ts">
import type { Addon, AddonState } from '@renderer/composables/useAddons'
import { ADDON_SETUP, useAddons } from '@renderer/composables/useAddons'
import { useAddonSwitch } from '@renderer/composables/useAddonSwitch'
import { useBoards } from '@renderer/composables/useBoards'
import { useIntiface } from '@renderer/composables/useIntiface'
import { useOpenShock } from '@renderer/composables/useOpenShock'
import { computed, ref } from 'vue'
import { useRouter } from 'vue-router'

// Settings > Add-ons: one card per add-on, grouped by what they're for. An add-on that's off opens
// no connection and adds nothing to the app; turning one on starts it right away.
const { addons, state, finishSetup } = useAddons()
const { setAddon } = useAddonSwitch()
const { hasToken } = useOpenShock()
const { boards } = useBoards()
const { installed: intifaceInstalled, launch: launchIntiface } = useIntiface()
const router = useRouter()

const GROUPS: Record<string, string> = { toys: 'Toys and shockers', movement: 'Movement', gear: 'Haptic gear', vr: 'In VR', assistants: 'AI assistants' }

const BADGES: Record<string, { label: string; color: 'error' | 'warning' | 'info' | 'neutral' }> = {
  shocks: { label: 'Shocks', color: 'error' },
  electrical: { label: 'Electrical', color: 'error' },
  beta: { label: 'Beta', color: 'info' },
  experimental: { label: 'Experimental', color: 'warning' }
}

const DOT: Record<AddonState, string> = {
  off: 'bg-(--ui-text-dimmed)',
  busy: 'bg-warning animate-pulse',
  ok: 'bg-success shadow-[0_0_8px_var(--ui-success)]',
  warn: 'bg-warning shadow-[0_0_8px_var(--ui-warning)]',
  error: 'bg-error'
}

const groups = computed(() => {
  const byGroup = new Map<string, Addon[]>()

  addons.value.forEach((addon) => byGroup.set(addon.group, [...(byGroup.get(addon.group) ?? []), addon]))

  return Array.from(byGroup, ([key, items]) => ({ key, label: GROUPS[key] ?? key, items }))
})

const onCount = computed(() => addons.value.filter((addon) => addon.enabled).length)

const kept = computed(() =>
  (state.value?.kept ?? []).map((id) => addons.value.find((addon) => addon.id === id)?.name ?? id)
)

const keptText = computed(() => {
  const names = kept.value

  return names.length > 1 ? `${names.slice(0, -1).join(', ')} and ${names.at(-1)}` : (names[0] ?? '')
})

// Turning one on that still needs something from you opens its settings: a key, a board.
const needsSetup = (addon: Addon): boolean =>
  (addon.id === 'openshock' && !hasToken.value) || (addon.id === 'boards' && boards.value.length === 0)

const switching = ref<string | null>(null)

const toggle = async (addon: Addon, enabled: boolean): Promise<void> => {
  switching.value = addon.id
  try {
    if ((await setAddon(addon, enabled)) && enabled && needsSetup(addon)) void router.push(ADDON_SETUP[addon.id] ?? '/settings/addons')
  } finally {
    switching.value = null
  }
}

const canLaunch = (addon: Addon): boolean =>
  addon.id === 'intiface' && addon.enabled && addon.status.state === 'warn' && intifaceInstalled.value
</script>

<template>
  <p class="text-[13px] text-muted">
    {{ onCount }} of {{ addons.length }} on. An add-on that's off opens no connection and adds nothing to the app.
    What it made is kept for when you turn it back on.
  </p>

  <UAlert
    v-if="kept.length"
    icon="i-lucide-puzzle"
    color="primary"
    variant="subtle"
    title="Add-ons are here"
    :description="`We kept ${keptText} on because you were using ${kept.length > 1 ? 'them' : 'it'}. Everything else starts off.`"
    :close="{ color: 'neutral', variant: 'link' }"
    @update:open="finishSetup(true)"
  />

  <section
    v-for="group in groups"
    :key="group.key"
    class="grid gap-2.5"
  >
    <h2 class="font-mono text-[11.5px] tracking-widest text-muted uppercase">
      {{ group.label }}
    </h2>
    <!-- As many columns as fit: the Add-ons section takes the whole width (see SettingsPage). -->
    <div class="grid grid-cols-[repeat(auto-fill,minmax(19rem,1fr))] gap-2.5">
      <article
        v-for="addon in group.items"
        :key="addon.id"
        class="grid content-start gap-2.5 rounded-panel border p-4.5 transition-colors"
        :class="addon.enabled ? 'border-primary/45 bg-primary/10' : 'border-transparent glass'"
      >
        <div class="flex items-center justify-between gap-3">
          <b class="text-[15.5px] font-semibold text-highlighted">{{ addon.name }}</b>
          <USwitch
            :model-value="addon.enabled"
            :loading="switching === addon.id"
            :aria-label="`${addon.name} on or off`"
            size="xl"
            @update:model-value="toggle(addon, !!$event)"
          />
        </div>

        <!-- The status (or what it does, while off), with its badge on the right. Add-ons this OS can't run aren't listed at all. -->
        <div class="flex items-start justify-between gap-3">
          <div
            v-if="addon.enabled"
            class="flex min-w-0 items-center gap-2 text-[13px] text-muted"
          >
            <span
              class="size-2.5 shrink-0 rounded-full"
              :class="DOT[addon.status.state]"
            />
            <span :class="addon.status.state === 'error' ? 'text-error' : undefined">{{ addon.status.text }}</span>
          </div>
          <p
            v-else
            class="min-w-0 text-[13px] text-muted"
          >
            {{ addon.pitch }}
          </p>

          <div
            v-if="addon.badges.length"
            class="flex shrink-0 flex-wrap justify-end gap-1.5"
          >
            <UBadge
              v-for="badge in addon.badges"
              :key="badge"
              :label="BADGES[badge]?.label ?? badge"
              :color="BADGES[badge]?.color ?? 'neutral'"
              variant="subtle"
              class="rounded-full"
            />
          </div>
        </div>

        <div
          v-if="addon.enabled"
          class="flex flex-wrap gap-2"
        >
          <UButton
            :to="ADDON_SETUP[addon.id] ?? '/settings/addons'"
            icon="i-lucide-settings-2"
            color="neutral"
            variant="subtle"
            class="rounded-2xl"
          >
            Set up
          </UButton>
          <UButton
            v-if="canLaunch(addon)"
            icon="i-lucide-rocket"
            color="neutral"
            variant="subtle"
            class="rounded-2xl"
            @click="launchIntiface"
          >
            Launch it
          </UButton>
        </div>
      </article>
    </div>
  </section>
</template>
