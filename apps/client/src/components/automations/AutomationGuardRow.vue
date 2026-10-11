<script setup lang="ts">
import ParameterListen from '@renderer/components/parameters/ParameterListen.vue'
import SegmentedControl from '@renderer/components/ui/SegmentedControl.vue'
import { useAvatarDetails } from '@renderer/composables/useAvatarDetails'
import { useLockedControls } from '@renderer/composables/useLockedControls'
import type { AutomationGuard, ParameterValueType } from '@renderer/lib/tauri-bridge'
import { isNoisyAddress } from '@renderer/utils/addressFilter'
import type { ConditionType } from '@renderer/utils/automations'
import { CONDITIONS, defaultCondition, GUARD_CONDITIONS, parameterName } from '@renderer/utils/automations'
import { computed } from 'vue'

// One "only if" of an automation: another parameter's state, the active profile, or whether
// viewers are on.
defineProps<{ otherAvatar: boolean }>()
const emit = defineEmits<{ remove: [] }>()
const guard = defineModel<AutomationGuard>({ required: true })

const { avatarDetails } = useAvatarDetails()
const { lockedControlGroups } = useLockedControls()

const kind = computed({
  get: () => guard.value.kind,
  set: (value: AutomationGuard['kind']) => {
    if (value === guard.value.kind) return
    guard.value =
      value === 'parameter'
        ? { kind: value, parameter: '', valueType: 'Bool', condition: { type: 'is-on' }, not: false }
        : value === 'profile'
          ? { kind: value, profileId: null }
          : { kind: value, online: true }
  }
})

// ---- Parameter

const param = computed(() => (guard.value.kind === 'parameter' ? guard.value : null))

const parameterItems = computed(() =>
  (avatarDetails.value?.parameters ?? [])
    .filter((p) => p.output && !isNoisyAddress(p.name))
    .map((p) => ({ label: `${p.name} · ${p.output!.type}`, value: p.output!.address, valueType: p.output!.type as ParameterValueType }))
    .sort((a, b) => a.label.localeCompare(b.label))
)

// Listen can hear parameters the picker hides; only those it lists can be checked.
const canPick = (address: string): boolean => parameterItems.value.some((p) => p.value === address)

const parameter = computed({
  get: () => param.value?.parameter || undefined,
  set: (address: string | undefined) => {
    if (!param.value || !address) return
    const valueType = parameterItems.value.find((p) => p.value === address)?.valueType ?? 'Bool'
    param.value.parameter = address
    if (param.value.valueType !== valueType) {
      param.value.valueType = valueType
      param.value.condition = defaultCondition(GUARD_CONDITIONS.find((c) => CONDITIONS[c].valueType === valueType)!)
    }
  }
})

const conditionValue = computed({
  get: () => (param.value && 'value' in param.value.condition ? param.value.condition.value : 0),
  set: (value: number) => {
    if (param.value && 'value' in param.value.condition) param.value.condition.value = value
  }
})

const verb = computed(() => {
  const type: ConditionType = param.value?.condition.type ?? 'is-on'
  const label = type === 'is-on' ? 'on' : type === 'is' ? 'equal to' : 'above'
  return [
    { value: 'is', label: `Is ${label}` },
    { value: 'not', label: `Isn't ${label}` }
  ]
})

const negated = computed({
  get: () => (param.value?.not ? 'not' : 'is'),
  set: (value: string) => {
    if (param.value) param.value.not = value === 'not'
  }
})

// USlider can hand back an array; only plain numbers are taken (see OpenShockFields.vue).
const fromSlider = (value: number | number[] | undefined): void => {
  if (typeof value === 'number') conditionValue.value = value
}

// ---- Profile

// USelect can't hold null, so No profile goes by a stand-in.
const NO_PROFILE = 'none'

const profileItems = computed(() => [
  { label: 'No profile', value: NO_PROFILE },
  ...lockedControlGroups.value.map((profile) => ({ label: profile.name, value: profile.id }))
])

const profile = computed({
  get: () => (guard.value.kind === 'profile' ? (guard.value.profileId ?? NO_PROFILE) : NO_PROFILE),
  set: (value: string) => {
    if (guard.value.kind === 'profile') guard.value.profileId = value === NO_PROFILE ? null : value
  }
})

// ---- Viewers

const viewers = computed({
  get: () => (guard.value.kind === 'viewers' && !guard.value.online ? 'none' : 'some'),
  set: (value: string) => {
    if (guard.value.kind === 'viewers') guard.value.online = value === 'some'
  }
})
</script>

<template>
  <div class="grid gap-3 rounded-field border border-default bg-(--aurora-glass) p-3.5">
    <div class="flex flex-wrap items-center gap-2">
      <SegmentedControl
        v-model="kind"
        :items="[
          { value: 'parameter', label: 'Parameter' },
          { value: 'profile', label: 'Profile' },
          { value: 'viewers', label: 'Viewers' }
        ]"
        aria-label="Only if"
        class="w-fit max-w-full flex-wrap"
      />
      <UButton
        icon="i-lucide-x"
        color="neutral"
        variant="ghost"
        size="sm"
        class="ml-auto"
        aria-label="Remove this condition"
        @click="emit('remove')"
      />
    </div>

    <template v-if="param">
      <UInput
        v-if="otherAvatar"
        :model-value="param.parameter ? `${parameterName(param.parameter)} · ${param.valueType}` : 'None picked'"
        class="w-full"
        disabled
      />
      <USelectMenu
        v-else
        v-model="parameter"
        :items="parameterItems"
        value-key="value"
        placeholder="Pick a parameter"
        class="w-full"
        virtualize
      />
      <ParameterListen
        v-if="!otherAvatar"
        :can-pick="canPick"
        @pick="parameter = $event"
      />
      <div
        v-if="param.parameter"
        class="flex flex-wrap items-center gap-3"
      >
        <SegmentedControl
          v-model="negated"
          :items="verb"
          aria-label="Is or isn't"
          class="w-fit"
        />
        <UInput
          v-if="param.valueType === 'Int'"
          v-model.number="conditionValue"
          type="number"
          step="1"
          class="w-28"
        />
        <div
          v-else-if="param.valueType === 'Float'"
          class="grid min-w-48 flex-1 gap-1"
        >
          <span class="text-sm text-muted tabular-nums">{{ conditionValue.toFixed(2) }}</span>
          <USlider
            :model-value="conditionValue"
            :min="0"
            :max="1"
            :step="0.01"
            @update:model-value="fromSlider"
          />
        </div>
      </div>
    </template>

    <template v-else-if="guard.kind === 'profile'">
      <UInput
        v-if="otherAvatar"
        model-value="A profile on that avatar"
        class="w-full"
        disabled
      />
      <USelect
        v-else
        v-model="profile"
        :items="profileItems"
        class="w-full max-w-80"
      />
    </template>

    <SegmentedControl
      v-else
      v-model="viewers"
      :items="[
        { value: 'some', label: 'Viewers are on' },
        { value: 'none', label: 'Nobody is watching' }
      ]"
      aria-label="Viewers"
      class="w-fit"
    />
  </div>
</template>
