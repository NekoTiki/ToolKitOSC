<script setup lang="ts">
import AutomationGuardRow from '@renderer/components/automations/AutomationGuardRow.vue'
import FormSection from '@renderer/components/ui/FormSection.vue'
import OptionGrid from '@renderer/components/ui/OptionGrid.vue'
import SegmentedControl from '@renderer/components/ui/SegmentedControl.vue'
import { useLockedControls } from '@renderer/composables/useLockedControls'
import type { AutomationGuard } from '@renderer/lib/tauri-bridge'
import { MAX_GUARDS } from '@renderer/utils/automations'
import { VR_MODE, YOU_STATES, youGuard, youState } from '@renderer/utils/builtinParameters'
import { computed } from 'vue'

// Easy mode, "only when": viewers and profile as plain choices, checks on the avatar as rows.
// Checked when it starts, like in Advanced.
defineProps<{ otherAvatar: boolean }>()
const guards = defineModel<AutomationGuard[]>({ required: true })
// One check is enough instead of all of them.
const any = defineModel<boolean | undefined>('any')

const { lockedControlGroups } = useLockedControls()

// One guard of a kind: replaced, or removed for "doesn't matter".
const setOnly = (kind: 'viewers' | 'profile', guard: AutomationGuard | null): void => {
  guards.value = [...guards.value.filter((g) => g.kind !== kind), ...(guard ? [guard] : [])]
}

const viewers = computed({
  get: () => {
    const guard = guards.value.find((g) => g.kind === 'viewers')
    return guard?.kind === 'viewers' ? (guard.online ? 'on' : 'off') : 'any'
  },
  set: (value: string) => setOnly('viewers', value === 'any' ? null : { kind: 'viewers', online: value === 'on' })
})

const ANY = 'any'
const NONE = 'none'
const profile = computed({
  get: () => {
    const guard = guards.value.find((g) => g.kind === 'profile')
    return guard?.kind === 'profile' ? (guard.profileId ?? NONE) : ANY
  },
  set: (value: string) => setOnly('profile', value === ANY ? null : { kind: 'profile', profileId: value === NONE ? null : value })
})
const profileItems = computed(() => [
  { value: ANY, label: 'Any' },
  { value: NONE, label: 'No profile' },
  ...lockedControlGroups.value.map((p) => ({ value: p.id, label: p.name }))
])

// What you're doing: one check on a VRChat parameter, as "state:yes" or "state:no".
const isYou = (guard: AutomationGuard): boolean => guard.kind === 'parameter' && (!!youState(guard.parameter) || guard.parameter === VR_MODE)
const you = computed({
  get: () => {
    const guard = guards.value.find(isYou)
    return guard?.kind === 'parameter' ? `${guard.parameter}:${guard.not ? 'no' : 'yes'}` : ANY
  },
  set: (value: string) => {
    const others = guards.value.filter((g) => !isYou(g))
    if (value === ANY) {
      guards.value = others
      return
    }
    const [address, yes] = value.split(':')
    guards.value = [...others, youGuard(address!, yes === 'yes')]
  }
})
const youItems = [
  { value: ANY, label: "Doesn't matter" },
  ...YOU_STATES.flatMap((s) => [
    { value: `${s.address}:yes`, label: `You're ${s.is}` },
    { value: `${s.address}:no`, label: `You're not ${s.is}` }
  ]),
  { value: `${VR_MODE}:yes`, label: "You're in VR" },
  { value: `${VR_MODE}:no`, label: "You're on desktop" }
]

// The rest are rows: parameters, time, avatar, chance.
const otherGuards = computed(() =>
  guards.value.flatMap((guard, index) => (guard.kind !== 'viewers' && guard.kind !== 'profile' && !isYou(guard) ? [index] : []))
)

const addCheck = (): void => {
  guards.value.push({ kind: 'parameter', parameter: '', valueType: 'Bool', condition: { type: 'is-on' }, not: false })
}
</script>

<template>
  <FormSection
    title="Only when…"
    description="Leave everything as it is to run any time. Checked when it starts."
  >
    <UFormField label="Viewers">
      <SegmentedControl
        v-model="viewers"
        :items="[
          { value: 'any', label: 'Doesn\'t matter' },
          { value: 'on', label: 'Someone is watching' },
          { value: 'off', label: 'Nobody is watching' }
        ]"
        class="w-fit max-w-full flex-wrap"
        accent
        aria-label="Viewers"
      />
    </UFormField>
    <UFormField label="Profile">
      <OptionGrid
        v-model="profile"
        :items="profileItems"
        aria-label="Profile"
      />
    </UFormField>
    <UFormField
      label="You"
      description="On every avatar, from what VRChat says."
    >
      <USelect
        v-model="you"
        :items="youItems"
        class="w-full max-w-80"
      />
    </UFormField>
    <UFormField
      label="More checks"
      description="Your avatar's parameters, the time, which avatar you wear, or a dice roll."
    >
      <div class="grid gap-2.5">
        <AutomationGuardRow
          v-for="index in otherGuards"
          :key="index"
          v-model="guards[index]!"
          :other-avatar="otherAvatar"
          @remove="guards.splice(index, 1)"
        />
        <div v-if="guards.length < MAX_GUARDS">
          <UButton
            icon="i-lucide-plus"
            color="neutral"
            variant="subtle"
            @click="addCheck"
          >
            Add a check
          </UButton>
        </div>
      </div>
    </UFormField>
    <UFormField
      v-if="guards.length > 1"
      label="How many have to be true?"
    >
      <SegmentedControl
        :model-value="any ? 'any' : 'all'"
        :items="[
          { value: 'all', label: 'All of them' },
          { value: 'any', label: 'Any one' }
        ]"
        class="w-fit"
        accent
        aria-label="How many have to be true?"
        @update:model-value="any = $event === 'any' || undefined"
      />
    </UFormField>
  </FormSection>
</template>
