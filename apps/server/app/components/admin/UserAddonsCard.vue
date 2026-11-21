<script setup lang="ts">
import type { AddonUsage } from '~/types/adminStats'
import { addonInfo } from '~/types/adminStats'

// Which add-ons this account's app has on, with device counts. What it last reported, not a history.
const props = defineProps<{ addons: AddonUsage[] }>()

// On first, then by name.
const rows = computed(() =>
  props.addons
    .map((addon) => {
      const { name, icon, devices: noun } = addonInfo(addon.addonId)

      return { ...addon, name, icon, noun }
    })
    .sort((a, b) => Number(b.enabled) - Number(a.enabled) || a.name.localeCompare(b.name))
)
</script>

<template>
  <UCard :ui="{ body: 'grid gap-3' }">
    <h2 class="text-base font-semibold text-highlighted">
      Add-ons
    </h2>
    <ul
      v-if="rows.length"
      class="grid gap-1.5 sm:grid-cols-2"
    >
      <li
        v-for="row in rows"
        :key="row.addonId"
        class="flex min-h-10 items-center gap-2.5 rounded-lg bg-(--aurora-well) px-3 text-[13px]"
        :class="{ 'text-dimmed': !row.enabled }"
      >
        <UIcon
          :name="row.icon"
          class="size-4 shrink-0"
        />
        <span class="flex-1 truncate">{{ row.name }}</span>
        <span
          v-if="row.enabled && row.devices !== null"
          class="font-mono text-xs text-muted tabular-nums"
        >{{ row.devices }} {{ row.noun ?? 'devices' }}</span>
        <UBadge
          :color="row.enabled ? 'success' : 'neutral'"
          variant="subtle"
          size="sm"
        >
          {{ row.enabled ? 'On' : 'Off' }}
        </UBadge>
      </li>
    </ul>
    <p
      v-else
      class="py-4 text-center text-sm text-muted"
    >
      Their app hasn't reported its add-ons yet. Older versions don't.
    </p>
  </UCard>
</template>
