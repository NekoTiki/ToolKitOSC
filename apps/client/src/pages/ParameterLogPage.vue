<script setup lang="ts">
import NumberBar from '@renderer/components/parameters/NumberBar.vue'
import EmptyState from '@renderer/components/ui/EmptyState.vue'
import { useAvatarDetails } from '@renderer/composables/useAvatarDetails'
import { PARAMETER_LOG_LIMIT, useParameterLog } from '@renderer/composables/useParameterLog'
import type { OSCArg } from '@renderer/env'
import { isNoisyAddress } from '@renderer/utils/addressFilter'
import { computed, ref } from 'vue'

// Parameters > Change log: every parameter value VRChat sent, newest first, capped at the last
// ~10k (see useParameterLog.ts, which records from app launch).
const PREFIX = '/avatar/parameters/'

const { entries, version, paused, clear } = useParameterLog()
const { avatarDetails } = useAvatarDetails()

// Declared type per address, so a float that happens to be 1 still shows as 1.00 on a 0 -> 1 bar.
const kinds = computed(() => {
  const map = new Map<string, 'Bool' | 'Float' | 'Int'>()

  for (const param of avatarDetails.value?.parameters ?? []) {
    if (param.output) map.set(param.output.address, param.output.type)
    if (param.input) map.set(param.input.address, param.input.type)
  }

  return map
})

const search = ref('')
const showNoisy = ref(false)

const nameOf = (address: string): string => (address.startsWith(PREFIX) ? address.slice(PREFIX.length) : address)

const rows = computed(() => {
  void version.value

  const query = search.value.trim().toLowerCase()
  const result = []
  const all = entries()

  // Walked backwards, so the newest entry comes first without copying and reversing 10k rows.
  for (let i = all.length - 1; i >= 0; i--) {
    const entry = all[i]!
    const name = nameOf(entry.address)

    if (!showNoisy.value && isNoisyAddress(name)) continue
    if (query && !name.toLowerCase().includes(query)) continue

    result.push({ ...entry, name })
  }

  return result
})

const time = new Intl.DateTimeFormat(undefined, { hour: '2-digit', minute: '2-digit', second: '2-digit', fractionalSecondDigits: 3, hour12: false })

const format = (value: OSCArg | undefined): string => {
  if (value === undefined) return '—'
  if (typeof value === 'boolean') return value ? 'True' : 'False'
  if (typeof value === 'number' && !Number.isInteger(value)) return value.toFixed(2)

  return String(value)
}

// Every row is exactly h-12 (48px, border included), so the virtualizer skips measuring them.
const VIRTUALIZE = { estimateSize: 48, skipMeasurement: true }

const COLUMNS = 'grid-cols-[6.5rem_minmax(0,1fr)_minmax(0,15rem)]'
</script>

<template>
  <PageLayout>
    <template #sidebar>
      <div class="flex min-h-9 shrink-0 items-center">
        <b class="text-xl font-semibold text-highlighted">Change log</b>
      </div>

      <UInput
        v-model="search"
        icon="i-lucide-search"
        placeholder="Filter by parameter"
        class="shrink-0"
        :ui="{ base: 'rounded-2xl' }"
      />

      <div class="flex min-h-0 flex-1 flex-col gap-3">
        <USwitch
          v-model="showNoisy"
          class="px-2.5"
          label="Show noisy parameters"
          description="Face tracking, PhysBone states…"
        />
      </div>

      <div class="grid shrink-0 gap-2">
        <UButton
          :icon="paused ? 'i-lucide-play' : 'i-lucide-pause'"
          :color="paused ? 'primary' : 'neutral'"
          variant="subtle"
          block
          @click="paused = !paused"
        >
          {{ paused ? 'Resume' : 'Pause' }}
        </UButton>
        <UButton
          icon="i-lucide-trash-2"
          color="neutral"
          variant="ghost"
          block
          @click="clear"
        >
          Clear
        </UButton>
        <p class="px-1 text-xs text-muted">
          Keeps the last {{ PARAMETER_LOG_LIMIT.toLocaleString() }} changes since the app started.
        </p>
      </div>
    </template>

    <template #header>
      <PageHeader
        title="Change log"
        :crumbs="[{ label: 'Parameters', to: '/parameters' }, { label: 'Change log' }]"
        :meta="`${rows.length.toLocaleString()} changes${paused ? ' · paused' : ' · live'}`"
      />
    </template>

    <div
      v-if="rows.length"
      class="flex h-full min-h-80 flex-col overflow-hidden rounded-field border border-default bg-(--aurora-glass)"
    >
      <div
        class="grid shrink-0 items-center gap-3 border-b border-default px-3.5 py-2.5 font-mono text-[11px] tracking-widest text-muted uppercase"
        :class="COLUMNS"
      >
        <span>Time</span>
        <span>Parameter</span>
        <span>Value</span>
      </div>
      <UScrollArea
        :items="rows"
        :virtualize="VIRTUALIZE"
        class="min-h-0 flex-1"
      >
        <template #default="{ item: row }">
          <div
            class="grid h-12 items-center gap-3 overflow-hidden border-b border-default px-3.5"
            :class="COLUMNS"
          >
            <span class="font-mono text-xs text-muted tabular-nums">{{ time.format(row.at) }}</span>
            <span
              class="truncate font-medium text-highlighted"
              :title="row.address"
            >{{ row.name }}</span>
            <span class="flex min-w-0 items-center gap-2 font-mono text-sm tabular-nums">
              <span class="truncate text-muted">{{ format(row.previous) }}</span>
              <UIcon
                name="i-lucide-arrow-right"
                class="size-3.5 shrink-0 text-dimmed"
              />
              <NumberBar
                v-if="typeof row.value === 'number'"
                class="text-highlighted"
                :value="row.value"
                :kind="kinds.get(row.address)"
              />
              <span
                v-else
                class="truncate text-highlighted"
              >{{ format(row.value) }}</span>
            </span>
          </div>
        </template>
      </UScrollArea>
    </div>

    <EmptyState
      v-else
      icon="i-lucide-history"
      :title="entries().length ? 'No changes match' : 'No changes yet'"
      :description="entries().length ? 'Try another filter, or turn on Show noisy parameters.' : 'Parameter changes from VRChat show up here as they arrive.'"
    />
  </PageLayout>
</template>
