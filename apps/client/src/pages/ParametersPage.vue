<script setup lang="ts">
import ParameterValue from '@renderer/components/parameters/ParameterValue.vue'
import EmptyState from '@renderer/components/ui/EmptyState.vue'
import { useAvatarDetails } from '@renderer/composables/useAvatarDetails'
import { useControls } from '@renderer/composables/useControls'
import { useOscMessages } from '@renderer/composables/useOscMessages'
import { usePresets } from '@renderer/composables/usePresets'
import { api } from '@renderer/lib/tauri-bridge'
import { isNoisyAddress } from '@renderer/utils/addressFilter'
import type { ControlType } from '@vrc-osc-toolkit/shared-ui'
import { computed, ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'

// Every parameter of the loaded avatar with its live value, which controls use it, and whether
// it's excluded from presets - one table replacing the parameters modal and the excluded
// parameters modal. Filters live in the sidebar; `?filter=` deep-links one (Presets links to
// the excluded list).
type Filter = 'all' | 'received' | 'missing' | 'used' | 'excluded'

const FILTERS: { value: Filter; label: string; icon: string }[] = [
  { value: 'all', label: 'All parameters', icon: 'i-lucide-list-tree' },
  { value: 'received', label: 'Received', icon: 'i-lucide-circle-check' },
  { value: 'missing', label: 'Not received yet', icon: 'i-lucide-clock' },
  { value: 'used', label: 'Used by controls', icon: 'i-lucide-layout-grid' },
  { value: 'excluded', label: 'Excluded from presets', icon: 'i-lucide-eye-off' }
]

const route = useRoute()
const router = useRouter()
const toast = useToast()
const { avatarDetails } = useAvatarDetails()
const { args } = useOscMessages()
const { controls } = useControls()
const { isExcluded, setExcluded } = usePresets()

const filter = computed<Filter>({
  get: () => (FILTERS.some((f) => f.value === route.query.filter) ? (route.query.filter as Filter) : 'all'),
  set: (value) => void router.replace({ query: value === 'all' ? {} : { filter: value } })
})

const search = ref('')
const showNoisy = ref(false)
const sortDesc = ref(false)
const pulling = ref(false)

// Which controls drive each input address, for the "Used by" column.
const usedBy = computed(() => {
  const map = new Map<string, string[]>()
  const add = (address: string | undefined, name: string): void => {
    if (!address) return
    map.set(address, [...(map.get(address) ?? []), name])
  }
  const collect = (control: ControlType): void => {
    switch (control.type) {
      case 'boolean':
      case 'enum':
      case 'step-enum':
      case 'slider':
        add(control.inputAddress, control.name)
        break
      case 'boolean-group':
        control.inputs.forEach((input) => add(input.inputAddress, control.name))
        break
      case 'boolean-enum':
        control.inputs.forEach((input) => [...input.inputAddress.true, ...input.inputAddress.false].forEach((address) => add(address, control.name)))
        break
    }
  }

  controls.value.forEach((group) => group.controls.forEach(collect))
  map.forEach((names, address) => map.set(address, [...new Set(names)]))

  return map
})

interface Row {
  name: string
  // What's shown and what values are read from: the output address, else the input one.
  address: string
  // Exclusion and "used by" are keyed by the input address (what gets written to).
  inputAddress?: string
  kind?: 'Bool' | 'Float' | 'Int'
  received: boolean
  noisy: boolean
  excluded: boolean
  usedBy: string[]
}

// Which addresses have a value at all. Object.keys only re-runs when an address is added or
// removed, not when a value changes - so the row list below isn't rebuilt on every OSC message
// (face tracking sends dozens a second). Live values render in ParameterValue instead.
const receivedAddresses = computed(() => new Set(Object.keys(args.value)))

const allRows = computed<Row[]>(() =>
  (avatarDetails.value?.parameters ?? []).map((param) => {
    const address = param.output?.address ?? param.input?.address ?? ''

    return {
      name: param.name,
      address,
      inputAddress: param.input?.address,
      kind: param.output?.type ?? param.input?.type,
      received: receivedAddresses.value.has(address),
      noisy: isNoisyAddress(param.name),
      excluded: !!param.input && isExcluded(param.input.address),
      usedBy: param.input ? (usedBy.value.get(param.input.address) ?? []) : []
    }
  })
)

const collator = new Intl.Collator(undefined, { sensitivity: 'base', numeric: true })

const visibleRows = computed(() => allRows.value.filter((row) => showNoisy.value || !row.noisy))

const counts = computed<Record<Filter, number>>(() => ({
  all: visibleRows.value.length,
  received: visibleRows.value.filter((row) => row.received).length,
  missing: visibleRows.value.filter((row) => !row.received).length,
  used: visibleRows.value.filter((row) => row.usedBy.length).length,
  excluded: visibleRows.value.filter((row) => row.excluded).length
}))

const noisyCount = computed(() => allRows.value.filter((row) => row.noisy).length)

const rows = computed(() => {
  const query = search.value.trim().toLowerCase()
  const matches = (row: Row): boolean =>
    ({ all: true, received: row.received, missing: !row.received, used: row.usedBy.length > 0, excluded: row.excluded })[filter.value]

  return visibleRows.value
    .filter((row) => matches(row) && (!query || row.name.toLowerCase().includes(query) || row.address.toLowerCase().includes(query)))
    .sort((a, b) => (sortDesc.value ? -1 : 1) * collator.compare(a.name, b.name))
})

const toggleExcluded = (row: Row): void => {
  if (!row.inputAddress) return

  setExcluded(row.inputAddress, !row.excluded)
  toast.add({
    title: row.excluded ? `${row.name} included in presets again` : `${row.name} excluded from presets`,
    icon: row.excluded ? 'i-lucide-eye' : 'i-lucide-eye-off'
  })
}

const forcePull = async (missingOnly: boolean): Promise<void> => {
  pulling.value = true

  try {
    const count = await api.forcePullParameters(missingOnly)

    toast.add({
      title: count > 0 ? `Pulled ${count} parameter${count === 1 ? '' : 's'} from VRChat` : 'Nothing to pull',
      description: count > 0 || !missingOnly ? undefined : 'Every visible parameter has already been received.',
      icon: 'i-lucide-download',
      color: 'success'
    })
  } catch (err) {
    toast.add({ title: 'Could not pull parameters', description: String(err), icon: 'i-lucide-triangle-alert', color: 'error' })
  } finally {
    pulling.value = false
  }
}

// Every row is exactly h-14.5 (58px, border included), so the virtualizer can skip measuring
// each one in the DOM.
const VIRTUALIZE = { estimateSize: 58, skipMeasurement: true }

const COLUMNS = 'grid-cols-[minmax(0,1fr)_4.5rem_7.5rem_minmax(0,12rem)_7.5rem] max-lg:grid-cols-[minmax(0,1fr)_4.5rem_7.5rem_7.5rem]'
</script>

<template>
  <PageLayout>
    <template #sidebar>
      <div class="flex min-h-9 shrink-0 items-center">
        <b class="text-xl font-semibold text-highlighted">Parameters</b>
      </div>

      <UInput
        v-model="search"
        icon="i-lucide-search"
        placeholder="Search name or address"
        class="shrink-0"
        :ui="{ base: 'rounded-2xl' }"
      />

      <nav
        aria-label="Filters"
        class="-mr-1.5 flex min-h-0 flex-1 flex-col gap-1.5 overflow-y-auto pr-1.5"
      >
        <button
          v-for="item in FILTERS"
          :key="item.value"
          type="button"
          class="grid min-h-14 cursor-pointer grid-cols-[auto_minmax(0,1fr)] items-center gap-x-2.5 rounded-2xl border px-2.5 py-2 text-left transition-colors"
          :class="filter === item.value ? 'border-primary/50 bg-primary/14' : 'border-transparent hover:bg-(--aurora-glass)'"
          :aria-pressed="filter === item.value"
          @click="filter = item.value"
        >
          <span
            class="row-span-2 grid size-8.5 place-items-center rounded-xl bg-(--aurora-well)"
            :class="filter === item.value ? 'text-primary' : 'text-muted'"
          >
            <UIcon
              :name="item.icon"
              class="size-4.5"
            />
          </span>
          <b class="truncate text-[14.5px] font-semibold text-highlighted">{{ item.label }}</b>
          <small class="col-start-2 text-xs text-muted">{{ counts[item.value] }} parameters</small>
        </button>

        <RouterLink
          to="/parameters/log"
          class="grid min-h-14 cursor-pointer grid-cols-[auto_minmax(0,1fr)] items-center gap-x-2.5 rounded-2xl border border-transparent px-2.5 py-2 text-left transition-colors hover:bg-(--aurora-glass)"
        >
          <span class="row-span-2 grid size-8.5 place-items-center rounded-xl bg-(--aurora-well) text-muted">
            <UIcon
              name="i-lucide-history"
              class="size-4.5"
            />
          </span>
          <b class="truncate text-[14.5px] font-semibold text-highlighted">Change log</b>
          <small class="col-start-2 text-xs text-muted">Every value as it arrives</small>
        </RouterLink>

        <USwitch
          v-model="showNoisy"
          class="mt-1 px-2.5"
          label="Show noisy parameters"
          :description="`${noisyCount} auto-generated ones (face tracking, PhysBone states…)`"
        />
      </nav>

      <div class="grid shrink-0 gap-2">
        <UButton
          icon="i-lucide-download"
          color="neutral"
          variant="subtle"
          block
          :loading="pulling"
          :disabled="!avatarDetails"
          @click="forcePull(true)"
        >
          Pull missing values
        </UButton>
        <UButton
          icon="i-lucide-refresh-cw"
          color="neutral"
          variant="ghost"
          block
          :disabled="pulling || !avatarDetails"
          @click="forcePull(false)"
        >
          Pull all
        </UButton>
        <p class="px-1 text-xs text-muted">
          Asks VRChat for values over OSCQuery.
        </p>
      </div>
    </template>

    <template #header>
      <PageHeader
        title="Parameters"
        :meta="avatarDetails ? `${counts.received} of ${counts.all} received · live` : undefined"
      />
    </template>

    <EmptyState
      v-if="!avatarDetails"
      icon="i-lucide-radio-tower"
      title="VRChat not detected"
      description="Parameters show up here once VRChat sends your avatar over OSC."
    />

    <div
      v-else-if="rows.length"
      class="flex h-full min-h-80 flex-col overflow-hidden rounded-field border border-default bg-(--aurora-glass)"
    >
      <div
        class="grid shrink-0 items-center gap-3 border-b border-default px-3.5 py-2.5 font-mono text-[11px] tracking-widest text-muted uppercase"
        :class="COLUMNS"
      >
        <button
          type="button"
          class="flex cursor-pointer items-center gap-1 uppercase"
          @click="sortDesc = !sortDesc"
        >
          Name
          <UIcon
            :name="sortDesc ? 'i-lucide-arrow-up' : 'i-lucide-arrow-down'"
            class="size-3.5"
          />
        </button>
        <span>Type</span>
        <span>Value</span>
        <span class="max-lg:hidden">Used by</span>
        <span />
      </div>
      <!-- Virtualized: an avatar can declare hundreds of parameters. Rows are absolutely
      positioned, so their spacing comes from each row's own border, not a gap. -->
      <UScrollArea
        :items="rows"
        :virtualize="VIRTUALIZE"
        class="min-h-0 flex-1"
      >
        <template #default="{ item: row }">
          <div
            class="grid h-14.5 items-center gap-3 overflow-hidden border-b border-default px-3.5 py-2"
            :class="COLUMNS"
          >
            <div class="grid min-w-0">
              <b class="truncate font-medium text-highlighted">{{ row.name }}</b>
              <small class="truncate font-mono text-[11.5px] text-muted">{{ row.address }}</small>
            </div>
            <span class="w-fit rounded-full bg-(--aurora-well) px-2 py-0.5 text-xs text-muted">{{ row.kind ?? '—' }}</span>
            <ParameterValue
              :address="row.address"
              :kind="row.kind"
            />
            <span
              class="truncate text-sm text-muted max-lg:hidden"
              :title="row.usedBy.join(', ')"
            >{{ row.usedBy.length ? row.usedBy.join(', ') : '—' }}</span>
            <UButton
              v-if="row.inputAddress"
              size="md"
              :color="row.excluded ? 'secondary' : 'neutral'"
              :variant="row.excluded ? 'soft' : 'subtle'"
              :icon="row.excluded ? 'i-lucide-eye' : 'i-lucide-eye-off'"
              class="justify-self-end"
              @click="toggleExcluded(row)"
            >
              {{ row.excluded ? 'Include' : 'Exclude' }}
            </UButton>
          </div>
        </template>
      </UScrollArea>
    </div>

    <EmptyState
      v-else
      icon="i-lucide-search"
      title="No parameters match"
      :description="showNoisy ? 'Try another search or filter.' : 'Try another search or filter, or turn on Show noisy parameters.'"
    />
  </PageLayout>
</template>
