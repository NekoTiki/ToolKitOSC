<script setup lang="ts">
import NumberBar from '@renderer/components/parameters/NumberBar.vue'
import EmptyState from '@renderer/components/ui/EmptyState.vue'
import { useAvatarDetails } from '@renderer/composables/useAvatarDetails'
import { useOscMessages } from '@renderer/composables/useOscMessages'
import { computed } from 'vue'

// The PhysBones view of the Parameters page: the `_IsGrabbed`, `_IsPosed`, `_Angle`, `_Stretch` and `_Squish`
// parameters VRChat drives for each PhysBone, grouped into one live card. There's no "pulled"
// parameter - a grabbed bone with some stretch is being pulled. Cards keep their (alphabetical)
// place when their state changes, so nothing jumps under a VR pointer.
const PREFIX = '/avatar/parameters/'
const SUFFIXES = ['_IsGrabbed', '_IsPosed', '_Angle', '_Stretch', '_Squish'] as const

type Suffix = (typeof SUFFIXES)[number]
type State = 'pulled' | 'grabbed' | 'posed' | 'idle'

const STATES: Record<State, { label: string; icon: string; class: string }> = {
  pulled: { label: 'Pulled', icon: 'i-lucide-move-diagonal', class: 'bg-primary/16 text-primary' },
  grabbed: { label: 'Grabbed', icon: 'i-lucide-hand-grab', class: 'bg-primary/16 text-primary' },
  posed: { label: 'Posed', icon: 'i-lucide-pin', class: 'bg-secondary/16 text-secondary' },
  idle: { label: 'Idle', icon: 'i-lucide-hand', class: 'bg-(--aurora-well) text-muted' }
}

const VALUES: { suffix: Suffix; label: string }[] = [
  { suffix: '_Angle', label: 'Angle' },
  { suffix: '_Stretch', label: 'Stretch' },
  { suffix: '_Squish', label: 'Squish' }
]

const props = defineProps<{ search: string }>()

const { avatarDetails } = useAvatarDetails()
const { args } = useOscMessages()

const collator = new Intl.Collator(undefined, { sensitivity: 'base', numeric: true })

// Each PhysBone's name and the address of each parameter it has. The avatar config may not list
// PhysBone parameters, so the addresses VRChat actually sent count too. Object.keys only re-runs
// this when an address is added or removed, not on every value change.
const groups = computed(() => {
  const addresses = new Set(Object.keys(args.value))
  for (const param of avatarDetails.value?.parameters ?? []) {
    if (param.output) addresses.add(param.output.address)
    if (param.input) addresses.add(param.input.address)
  }

  const map = new Map<string, Partial<Record<Suffix, string>>>()
  for (const address of addresses) {
    const name = address.startsWith(PREFIX) ? address.slice(PREFIX.length) : ''
    const suffix = SUFFIXES.find((s) => name.endsWith(s) && name.length > s.length)
    if (!suffix) continue

    const base = name.slice(0, -suffix.length)
    map.set(base, { ...map.get(base), [suffix]: address })
  }

  return [...map].map(([name, addresses]) => ({ name, addresses })).sort((a, b) => collator.compare(a.name, b.name))
})

const bones = computed(() =>
  groups.value.map(({ name, addresses }) => {
    const value = (suffix: Suffix): unknown => (addresses[suffix] ? args.value[addresses[suffix]]?.[0] : undefined)
    const grabbed = value('_IsGrabbed') === true
    const posed = value('_IsPosed') === true
    const stretch = value('_Stretch')
    const state: State = grabbed ? (typeof stretch === 'number' && stretch > 0 ? 'pulled' : 'grabbed') : posed ? 'posed' : 'idle'

    return {
      name,
      state,
      // Grabbed wins the chip, but a bone can be both - the pose sticks once it's let go.
      alsoPosed: grabbed && posed,
      values: VALUES.map((item) => {
        const current = value(item.suffix)
        return { ...item, has: !!addresses[item.suffix], value: typeof current === 'number' ? current : undefined }
      })
    }
  })
)

// For the page header.
defineExpose({
  total: computed(() => bones.value.length),
  grabbed: computed(() => bones.value.filter((bone) => bone.state === 'grabbed' || bone.state === 'pulled').length)
})

const visible = computed(() => {
  const query = props.search.trim().toLowerCase()

  return bones.value.filter((bone) => !query || bone.name.toLowerCase().includes(query))
})
</script>

<template>
  <div
    v-if="visible.length"
    class="grid grid-cols-[repeat(auto-fill,minmax(17rem,1fr))] gap-3"
  >
    <article
      v-for="bone in visible"
      :key="bone.name"
      class="grid content-start gap-3 rounded-panel border glass p-4 transition-colors"
      :class="bone.state === 'grabbed' || bone.state === 'pulled' ? 'border-primary/50' : 'border-transparent'"
    >
      <header class="flex min-w-0 items-center gap-2">
        <b
          class="min-w-0 flex-1 truncate text-[14.5px] font-semibold text-highlighted"
          :title="bone.name"
        >{{ bone.name }}</b>
        <span
          v-if="bone.alsoPosed"
          class="shrink-0 rounded-full px-2 py-0.5 text-xs font-medium"
          :class="STATES.posed.class"
        >Posed</span>
        <span
          class="flex shrink-0 items-center gap-1.5 rounded-full px-2.5 py-1 text-xs font-medium"
          :class="STATES[bone.state].class"
        >
          <UIcon
            :name="STATES[bone.state].icon"
            class="size-3.5"
          />
          {{ STATES[bone.state].label }}
        </span>
      </header>

      <dl class="grid grid-cols-[4rem_minmax(0,1fr)] items-center gap-x-3 gap-y-2">
        <template
          v-for="item in bone.values"
          :key="item.suffix"
        >
          <dt class="text-sm text-muted">
            {{ item.label }}
          </dt>
          <dd class="min-w-0">
            <span
              v-if="!item.has"
              class="text-sm text-dimmed"
              title="This PhysBone doesn't have this parameter"
            >—</span>
            <NumberBar
              v-else-if="item.value !== undefined"
              :value="item.value"
              kind="Float"
            />
            <span
              v-else
              class="text-sm text-dimmed"
            >Not received</span>
          </dd>
        </template>
      </dl>
    </article>
  </div>

  <EmptyState
    v-else-if="!bones.length"
    icon="i-lucide-bone"
    title="No PhysBone parameters yet"
    description="Set a Parameter name on a VRC PhysBone component, then grab it in game. Its grab, pose, angle, stretch and squish show up here."
  />
  <EmptyState
    v-else
    icon="i-lucide-search-x"
    title="No PhysBones match"
    description="Try another search, or clear it."
  />
</template>
