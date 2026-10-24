<script setup lang="ts">
// Which app versions and systems the accounts seen in the last 30 days run.
interface Row {
  label: string | null
  value: number
}

const { redirectIfUnauthorized } = useAdminApi()

const { data, error } = await useFetch<{ versions: Row[]; systems: Row[] }>('/api/admin/app-versions')

if (error.value) await redirectIfUnauthorized(error.value)

const OS_NAMES: Record<string, string> = { windows: 'Windows', linux: 'Linux', macos: 'macOS' }

// Newest version on top; numeric compare so 1.10 comes after 1.9.
const versions = computed(() =>
  (data.value?.versions ?? [])
    .map((row) => ({ label: row.label ?? 'Unknown', value: row.value }))
    .sort((a, b) => b.label.localeCompare(a.label, 'en', { numeric: true }))
)
const systems = computed(() => (data.value?.systems ?? []).map((row) => ({ label: OS_NAMES[row.label ?? ''] ?? row.label ?? 'Unknown', value: row.value })))
</script>

<template>
  <UCard :ui="{ body: 'grid gap-4 md:grid-cols-2' }">
    <div class="grid content-start gap-3">
      <h2 class="text-base font-semibold text-highlighted">
        App versions <small class="font-normal text-muted">· last 30 days</small>
      </h2>
      <AdminRankedBars :rows="versions" />
    </div>
    <div class="grid content-start gap-3">
      <h2 class="text-base font-semibold text-highlighted">
        Systems
      </h2>
      <AdminRankedBars :rows="systems" />
    </div>
  </UCard>
</template>
