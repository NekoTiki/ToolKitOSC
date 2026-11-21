<script setup lang="ts">
import { addonInfo } from '~/types/adminStats'

// Which add-ons the accounts seen in the last 30 days have on, and how many devices they connect.
interface Row {
  addonId: string
  reported: number
  enabled: number
  devices: number
  withDevices: number
}

const { redirectIfUnauthorized } = useAdminApi()

const { data, error } = await useFetch<Row[]>('/api/admin/addon-stats')

if (error.value) await redirectIfUnauthorized(error.value)

const rows = computed(() => data.value ?? [])
// Every app reports every add-on its OS can run, so the most-reported one is how many apps reported.
const apps = computed(() => Math.max(0, ...rows.value.map((row) => row.reported)))

const enabled = computed(() =>
  rows.value
    .filter((row) => row.enabled)
    .map((row) => ({ label: addonInfo(row.addonId).name, value: row.enabled }))
    .sort((a, b) => b.value - a.value)
)
// Among the accounts that have at least one, so setups that never connected one don't count.
const devices = computed(() =>
  rows.value
    .filter((row) => row.withDevices)
    .map((row) => ({ label: addonInfo(row.addonId).name, value: row.devices / row.withDevices }))
    .sort((a, b) => b.value - a.value)
)

const oneDecimal = (value: number): string => value.toLocaleString('en-US', { maximumFractionDigits: 1 })
</script>

<template>
  <UCard :ui="{ body: 'grid gap-4 md:grid-cols-2' }">
    <div class="grid content-start gap-3">
      <h2 class="text-base font-semibold text-highlighted">
        Add-ons on <small class="font-normal text-muted">· {{ apps.toLocaleString('en-US') }} apps, last 30 days</small>
      </h2>
      <AdminRankedBars
        :rows="enabled"
        :max="apps"
      />
    </div>
    <div class="grid content-start gap-3">
      <h2 class="text-base font-semibold text-highlighted">
        Devices per user <small class="font-normal text-muted">· who has any</small>
      </h2>
      <AdminRankedBars
        :rows="devices"
        :format="oneDecimal"
      />
    </div>
  </UCard>
</template>
