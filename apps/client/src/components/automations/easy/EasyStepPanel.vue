<script setup lang="ts">
import BhapticsStepFields from '@renderer/components/automations/BhapticsStepFields.vue'
import OwoStepFields from '@renderer/components/automations/OwoStepFields.vue'
import FormSection from '@renderer/components/ui/FormSection.vue'
import OptionGrid from '@renderer/components/ui/OptionGrid.vue'
import SegmentedControl from '@renderer/components/ui/SegmentedControl.vue'
import { useAddons } from '@renderer/composables/useAddons'
import { useAutomations } from '@renderer/composables/useAutomations'
import { useAvatarDetails } from '@renderer/composables/useAvatarDetails'
import { useBoards } from '@renderer/composables/useBoards'
import { useControls } from '@renderer/composables/useControls'
import { useLockedControls } from '@renderer/composables/useLockedControls'
import type { AutomationStep, ParameterValueType } from '@renderer/lib/tauri-bridge'
import { isNoisyAddress } from '@renderer/utils/addressFilter'
import type { ConditionType, ControlRunType, OutputActionType, ParameterActionType } from '@renderer/utils/automations'
import {
  actionsFor,
  CHATBOX_MAX,
  controlRunsFor,
  defaultControlRun,
  defaultOutput,
  defaultParameterAction,
  newStep,
  OUTPUT_ACTIONS,
  PARAMETER_ACTIONS,
  parameterActionsFor,
  STEP_KINDS,
  stepCanWait,
  stepKindsFor
} from '@renderer/utils/automations'
import { EASY_DELAYS, EASY_PULSES, seconds } from '@renderer/utils/automationSentence'
import { computed } from 'vue'

// Easy mode, one thing the automation does: what kind of thing, which one, and how, in plain
// words. Each choice writes the same step the Advanced editor's AutomationStepCard edits.
const props = defineProps<{
  index: number
  condition: ConditionType
  // The trigger's address: a step can't set it, or it would set itself off.
  trigger: string
  canRemove: boolean
}>()
const emit = defineEmits<{ remove: [] }>()
const step = defineModel<AutomationStep>({ required: true })

const { avatarDetails } = useAvatarDetails()
const { describeContext } = useAutomations()
const { controls } = useControls()
const { lockedControlGroups } = useLockedControls()
const { boards } = useBoards()
const { offAddonForAutomationKind, offAddonForControlType } = useAddons()

const EASY_KINDS: Record<AutomationStep['kind'], string> = {
  control: 'Use a control',
  'avatar-parameter': 'Change your avatar',
  chatbox: 'Write in the chatbox',
  profile: 'Switch profile',
  'stop-everything': 'Stop everything',
  'board-output': 'Switch something on a board',
  'leash-enabled': 'Turn the leash on or off',
  'bhaptics-pulse': 'Buzz bHaptics motors',
  'bhaptics-effect': 'Play a bHaptics effect',
  'owo-sensation': 'Feel it on your OWO Skin'
}

// The kinds that can run on this trigger, plus the current one so it still shows what it is.
const kindItems = computed(() => {
  const fitting = stepKindsFor(props.condition, (kind) => !offAddonForAutomationKind(kind))
  return (fitting.includes(step.value.kind) ? fitting : [...fitting, step.value.kind]).map((kind) => ({
    value: kind,
    label: EASY_KINDS[kind],
    icon: STEP_KINDS[kind].icon
  }))
})

const kind = computed({
  get: () => step.value.kind,
  set: (value: AutomationStep['kind']) => {
    if (value === step.value.kind) return
    const next = newStep(value, props.condition)
    step.value = { ...next, delayMs: stepCanWait(next) ? step.value.delayMs : 0 }
  }
})

// "How long after the start?": the first step only shows it once it waits.
const delay = computed({
  get: () => String(step.value.delayMs),
  set: (value: string) => (step.value.delayMs = Number(value))
})
const delayItems = EASY_DELAYS.map((ms) => ({ value: String(ms), label: ms ? `${seconds(ms)} later` : 'Right away' }))

// USlider can hand back an array; only plain numbers are taken (see OpenShockFields.vue).
const fromSlider = (value: number | number[] | undefined, apply: (value: number) => void): void => {
  if (typeof value === 'number') apply(value)
}

// One way to run a control or set a parameter, in plain words. On/off pairs for Bools.
interface Way {
  key: string
  label: string
  type: string
  on?: boolean
}
const waysFor = (types: string[], bool: boolean, labels: Record<string, string>): Way[] =>
  types.flatMap((type) =>
    bool && (type === 'set' || type === 'hold')
      ? [
          { key: `${type}-on`, label: type === 'set' ? 'On' : 'On while it lasts', type, on: true },
          { key: `${type}-off`, label: type === 'set' ? 'Off' : 'Off while it lasts', type, on: false }
        ]
      : [{ key: type, label: labels[type] ?? type, type }]
  )

// ---- Use a control

const controlStep = computed(() => (step.value.kind === 'control' ? step.value : null))

const allControls = computed(() => controls.value.flatMap((group) => group.controls.map((control) => ({ group, control }))))

const picked = computed(() =>
  controlStep.value ? allControls.value.find((c) => c.group.id === controlStep.value!.groupId && c.control.id === controlStep.value!.controlId)?.control : undefined
)

const controlItems = computed(() =>
  allControls.value
    .filter(({ control }) => controlRunsFor(props.condition, control.type).length)
    .filter(({ control }) => control === picked.value || !offAddonForControlType(control.type))
    .map(({ group, control }) => ({ label: `${control.name} · ${group.name}`, value: `${group.id}:${control.id}` }))
)

const controlTarget = computed({
  get: () => (controlStep.value?.controlId ? `${controlStep.value.groupId}:${controlStep.value.controlId}` : undefined),
  set: (value: string | undefined) => {
    if (!controlStep.value || !value) return
    const [groupId, controlId] = value.split(':')
    const control = allControls.value.find((c) => c.group.id === groupId && c.control.id === controlId)?.control
    if (!control) return
    Object.assign(controlStep.value, { groupId, controlId, controlType: control.type, run: defaultControlRun(controlRunsFor(props.condition, control.type)[0]!, control) })
  }
})

const valueKind = computed((): 'bool' | 'choice' | 'level' => {
  switch (controlStep.value?.controlType) {
    case 'boolean':
    case 'boolean-group':
      return 'bool'
    case 'slider':
    case 'intiface-toy':
      return 'level'
    default:
      return 'choice'
  }
})

const controlWays = computed(() => {
  if (!controlStep.value) return []
  const type = controlStep.value.controlType
  const press = picked.value?.type === 'open-shock-shocker' ? (picked.value.mode === 'Vibrate' ? 'Vibrate' : 'Shock') : 'Apply'
  return waysFor(controlRunsFor(props.condition, type), valueKind.value === 'bool', {
    set: type === 'intiface-toy' ? 'Leave it on' : 'Set it',
    toggle: 'The other way',
    pulse: 'For a moment',
    press,
    hold: 'For as long as it lasts',
    follow: 'Following it'
  })
})

const controlWay = computed({
  get: () => {
    const run = controlStep.value?.run
    if (!run) return ''
    return valueKind.value === 'bool' && (run.type === 'set' || run.type === 'hold') ? `${run.type}-${run.value ? 'on' : 'off'}` : run.type
  },
  set: (key: string) => {
    const way = controlWays.value.find((w) => w.key === key)
    if (!controlStep.value || !way) return
    const before = controlStep.value.run
    const run = defaultControlRun(way.type as ControlRunType, picked.value)
    // A level carries over between leave on, for a moment and while it lasts.
    const level = 'value' in before && typeof before.value === 'number' ? before.value : undefined
    if ('value' in run) run.value = way.on ?? (valueKind.value === 'level' && level !== undefined ? level : run.value)
    controlStep.value.run = run
    if (!stepCanWait(controlStep.value)) controlStep.value.delayMs = 0
  }
})

const level = computed({
  get: () => {
    const run = controlStep.value?.run
    return run && 'value' in run && typeof run.value === 'number' ? run.value : 0
  },
  set: (value: number) => {
    const run = controlStep.value?.run
    if (run && 'value' in run) run.value = value
  }
})

const pulseMs = computed({
  get: () => String(controlStep.value?.run.type === 'pulse' ? controlStep.value.run.ms : 2000),
  set: (value: string) => {
    if (controlStep.value?.run.type === 'pulse') controlStep.value.run.ms = Number(value)
  }
})
const pulseItems = EASY_PULSES.map((ms) => ({ value: String(ms), label: seconds(ms) }))

// Enum options and Toggle Logic inputs go by index; patterns by id.
const choiceItems = computed((): { label: string; value: string }[] => {
  const control = picked.value
  switch (control?.type) {
    case 'enum':
    case 'step-enum':
      return control.options.map((option, index) => ({ label: option.name, value: String(index) }))
    case 'boolean-enum':
      return control.inputs.map((input, index) => ({ label: input.name, value: String(index) }))
    case 'intiface-pattern':
      return [{ label: 'Off', value: 'off' }, ...control.allowedPatterns.map((pattern) => ({ label: pattern.name, value: pattern.id }))]
    default:
      return []
  }
})

const choice = computed({
  get: () => {
    const run = controlStep.value?.run
    return run && (run.type === 'set' || run.type === 'hold') ? String(run.value) : ''
  },
  set: (value: string) => {
    const run = controlStep.value?.run
    if (run && (run.type === 'set' || run.type === 'hold')) run.value = controlStep.value!.controlType === 'intiface-pattern' ? value : Number(value)
  }
})

const showLevel = computed(() => valueKind.value === 'level' && ['set', 'hold', 'pulse'].includes(controlStep.value?.run.type ?? ''))
const showChoice = computed(() => valueKind.value === 'choice' && ['set', 'hold'].includes(controlStep.value?.run.type ?? ''))

// ---- Change your avatar

const param = computed(() => (step.value.kind === 'avatar-parameter' ? step.value : null))

// Only parameters VRChat accepts input for, minus the trigger.
const parameterItems = computed(() =>
  (avatarDetails.value?.parameters ?? [])
    .filter((p) => p.input && p.input.address !== props.trigger && !isNoisyAddress(p.name))
    .filter((p) => parameterActionsFor(props.condition, p.input!.type as ParameterValueType).length)
    .map((p) => ({
      label: describeContext.value.controlForParameter?.(p.input!.address)?.name ?? p.name,
      value: p.input!.address,
      valueType: p.input!.type as ParameterValueType
    }))
    .sort((a, b) => a.label.localeCompare(b.label))
)

const parameter = computed({
  get: () => param.value?.parameter || undefined,
  set: (address: string | undefined) => {
    if (!param.value || !address) return
    const valueType = parameterItems.value.find((p) => p.value === address)?.valueType ?? 'Bool'
    param.value.parameter = address
    if (param.value.valueType !== valueType || !PARAMETER_ACTIONS[param.value.set.type].valueTypes.includes(valueType)) {
      param.value.valueType = valueType
      param.value.set = defaultParameterAction(parameterActionsFor(props.condition, valueType)[0]!, valueType)
    }
  }
})

const paramWays = computed(() =>
  param.value
    ? waysFor(parameterActionsFor(props.condition, param.value.valueType), param.value.valueType === 'Bool', {
        set: 'Set it',
        toggle: 'The other way',
        hold: 'For as long as it lasts',
        follow: 'Following it'
      })
    : []
)

const paramWay = computed({
  get: () => {
    const set = param.value?.set
    if (!set) return ''
    return param.value!.valueType === 'Bool' && (set.type === 'set' || set.type === 'hold') ? `${set.type}-${set.value ? 'on' : 'off'}` : set.type
  },
  set: (key: string) => {
    const way = paramWays.value.find((w) => w.key === key)
    if (!param.value || !way) return
    const set = defaultParameterAction(way.type as ParameterActionType, param.value.valueType)
    if ('value' in set && way.on !== undefined) set.value = way.on ? 1 : 0
    param.value.set = set
    if (!stepCanWait(param.value)) param.value.delayMs = 0
  }
})

const paramValue = computed({
  get: () => (param.value && 'value' in param.value.set ? param.value.set.value : 0),
  set: (value: number) => {
    if (param.value && 'value' in param.value.set) param.value.set.value = value
  }
})

// An Int's options, named by the control that drives it when there is one.
const paramOptions = computed(() => {
  const control = param.value?.parameter ? describeContext.value.controlForParameter?.(param.value.parameter) : undefined
  return control?.type === 'enum' || control?.type === 'step-enum' ? control.options.map((o) => ({ value: String(o.value), label: o.name })) : null
})

// ---- Profile

const profileStep = computed(() => (step.value.kind === 'profile' ? step.value : null))
const NO_PROFILE = 'none'
const profileItems = computed(() => [
  { label: 'No profile', value: NO_PROFILE },
  ...lockedControlGroups.value.map((profile) => ({ label: profile.name, value: profile.id }))
])
const profileTarget = computed({
  get: () => profileStep.value?.profileId ?? NO_PROFILE,
  set: (value: string) => {
    if (profileStep.value) profileStep.value.profileId = value === NO_PROFILE ? null : value
  }
})

// ---- Switch something on a board

const board = computed(() => (step.value.kind === 'board-output' ? step.value : null))

const boardItems = computed(() =>
  boards.value.flatMap((b) =>
    (b.config ?? []).map((output) => ({ label: `${output.label} · ${b.name}${b.status === 'online' ? '' : ' (offline)'}`, value: `${b.id}:${output.pin}` }))
  )
)

// The picked output as saved on its board, for its max on-time, and whether it can dim.
const boardOutput = computed(() => (board.value ? boards.value.find((b) => b.id === board.value!.boardId)?.config?.find((o) => o.pin === board.value!.pin) : undefined))
const boardPwm = computed(() => !!board.value && !!boards.value.find((b) => b.id === board.value!.boardId)?.outputs.some((p) => p.pin === board.value!.pin && p.pwm))

// A pulse never outlasts the output's max on-time: the board would cut it short.
const fitPulse = (): void => {
  const output = board.value?.output
  if (output?.type === 'pulse' && boardOutput.value && output.onMs > boardOutput.value.maxOnMs) output.onMs = boardOutput.value.maxOnMs
}

const boardTarget = computed({
  get: () => (board.value && board.value.pin >= 0 ? `${board.value.boardId}:${board.value.pin}` : undefined),
  set: (value: string | undefined) => {
    if (!board.value || !value) return
    const [boardId, pin] = value.split(':')
    board.value.boardId = boardId!
    board.value.pin = Number(pin)
    fitPulse()
  }
})

const BOARD_WAYS: Record<OutputActionType, string> = {
  pulse: 'For a moment',
  toggle: 'The other way',
  off: 'Off',
  hold: 'For as long as it lasts',
  follow: 'Following it'
}
const boardWays = computed(() =>
  actionsFor(props.condition)
    .filter((type) => !OUTPUT_ACTIONS[type].needsPwm || boardPwm.value)
    .map((type) => ({ value: type, label: BOARD_WAYS[type] }))
)

const boardWay = computed({
  get: (): OutputActionType => board.value?.output.type ?? 'pulse',
  set: (type: OutputActionType) => {
    if (!board.value) return
    board.value.output = defaultOutput(type)
    if (type === 'follow') board.value.chatbox = false
    if (!stepCanWait(board.value)) board.value.delayMs = 0
    fitPulse()
  }
})

const boardPulse = computed({
  get: () => String(board.value?.output.type === 'pulse' ? board.value.output.onMs : 1000),
  set: (value: string) => {
    if (board.value?.output.type === 'pulse') board.value.output.onMs = Number(value)
  }
})
const boardPulseItems = computed(() =>
  EASY_PULSES.filter((ms) => !boardOutput.value || ms <= boardOutput.value.maxOnMs).map((ms) => ({ value: String(ms), label: seconds(ms) }))
)

// ---- Leash, bHaptics, OWO

const leashStep = computed(() => (step.value.kind === 'leash-enabled' ? step.value : null))
const leashOn = computed({
  get: () => (leashStep.value?.enabled ? 'on' : 'off'),
  set: (value: string) => {
    if (leashStep.value) leashStep.value.enabled = value === 'on'
  }
})
const bhapticsStep = computed(() => (step.value.kind === 'bhaptics-pulse' || step.value.kind === 'bhaptics-effect' ? step.value : null))
const owoStep = computed(() => (step.value.kind === 'owo-sensation' ? step.value : null))
</script>

<template>
  <FormSection :title="index ? 'And then…' : 'What should happen?'">
    <template
      v-if="canRemove"
      #actions
    >
      <UButton
        icon="i-lucide-x"
        color="neutral"
        variant="ghost"
        @click="emit('remove')"
      >
        Remove
      </UButton>
    </template>

    <OptionGrid
      v-model="kind"
      :items="kindItems"
      min-width="12rem"
      aria-label="What should happen"
    />

    <template v-if="controlStep">
      <UFormField label="Which control?">
        <USelectMenu
          v-model="controlTarget"
          :items="controlItems"
          value-key="value"
          placeholder="Pick a control"
          class="w-full"
        />
      </UFormField>
      <template v-if="controlStep.controlId">
        <UFormField
          v-if="controlWays.length > 1"
          label="How?"
        >
          <SegmentedControl
            v-model="controlWay"
            :items="controlWays.map((w) => ({ value: w.key, label: w.label }))"
            class="w-fit max-w-full flex-wrap"
            accent
            aria-label="How?"
          />
        </UFormField>
        <p
          v-if="picked?.type === 'open-shock-shocker'"
          class="text-sm text-muted"
        >
          Uses the shocker's own intensity, duration and cooldown.
        </p>
        <UFormField
          v-if="showLevel"
          :label="`${controlStep.controlType === 'intiface-toy' ? 'How strong' : 'Value'}: ${Math.round(level * 100)}%`"
        >
          <USlider
            :model-value="level"
            :min="0"
            :max="1"
            :step="0.01"
            @update:model-value="fromSlider($event, (value) => (level = value))"
          />
        </UFormField>
        <UFormField
          v-if="controlStep.run.type === 'pulse'"
          label="For how long?"
        >
          <SegmentedControl
            v-model="pulseMs"
            :items="pulseItems"
            class="w-fit"
            aria-label="For how long?"
          />
        </UFormField>
        <UFormField
          v-if="showChoice"
          label="Which one?"
        >
          <OptionGrid
            v-model="choice"
            :items="choiceItems"
            aria-label="Which one?"
          />
        </UFormField>
      </template>
    </template>

    <template v-else-if="param">
      <UFormField label="What on your avatar?">
        <USelectMenu
          v-model="parameter"
          :items="parameterItems"
          value-key="value"
          placeholder="Pick a parameter"
          class="w-full"
          virtualize
        />
      </UFormField>
      <template v-if="param.parameter">
        <UFormField
          v-if="paramWays.length > 1"
          label="How?"
        >
          <SegmentedControl
            v-model="paramWay"
            :items="paramWays.map((w) => ({ value: w.key, label: w.label }))"
            class="w-fit max-w-full flex-wrap"
            accent
            aria-label="How?"
          />
        </UFormField>
        <UFormField
          v-if="param.valueType === 'Int' && 'value' in param.set"
          label="To"
        >
          <OptionGrid
            v-if="paramOptions"
            :model-value="String(paramValue)"
            :items="paramOptions"
            aria-label="To"
            @update:model-value="paramValue = Number($event)"
          />
          <UInput
            v-else
            v-model.number="paramValue"
            type="number"
            step="1"
            min="0"
            max="255"
            class="max-w-40"
          />
        </UFormField>
        <UFormField
          v-else-if="param.valueType === 'Float' && 'value' in param.set"
          :label="`To ${Math.round(paramValue * 100)}%`"
        >
          <USlider
            :model-value="paramValue"
            :min="0"
            :max="1"
            :step="0.01"
            @update:model-value="fromSlider($event, (value) => (paramValue = value))"
          />
        </UFormField>
      </template>
    </template>

    <UFormField
      v-else-if="step.kind === 'chatbox'"
      label="Message"
    >
      <UInput
        v-model="step.text"
        :maxlength="CHATBOX_MAX"
        placeholder="Hi, welcome in!"
        class="w-full"
      />
    </UFormField>

    <template v-else-if="profileStep">
      <UFormField label="Which profile?">
        <OptionGrid
          v-model="profileTarget"
          :items="profileItems"
          aria-label="Which profile?"
        />
      </UFormField>
      <p
        v-if="profileStep.whileHeld"
        class="text-sm text-muted"
      >
        Goes back to the profile you had when it ends.
      </p>
    </template>

    <p
      v-else-if="step.kind === 'stop-everything'"
      class="text-sm text-muted"
    >
      Everything stops, toys and shockers included. Only you can turn it back on.
    </p>

    <template v-else-if="board">
      <UFormField label="Which output?">
        <USelectMenu
          v-model="boardTarget"
          :items="boardItems"
          value-key="value"
          placeholder="Pick an output"
          class="w-full"
        />
      </UFormField>
      <template v-if="board.pin >= 0">
        <UFormField
          v-if="boardWays.length > 1"
          label="How?"
        >
          <SegmentedControl
            v-model="boardWay"
            :items="boardWays"
            class="w-fit max-w-full flex-wrap"
            accent
            aria-label="How?"
          />
        </UFormField>
        <UFormField
          v-if="board.output.type === 'pulse'"
          label="For how long?"
          :description="boardOutput ? `Up to ${seconds(boardOutput.maxOnMs)}, its max on-time.` : undefined"
        >
          <SegmentedControl
            v-model="boardPulse"
            :items="boardPulseItems"
            class="w-fit"
            aria-label="For how long?"
          />
        </UFormField>
        <USwitch
          v-if="board.output.type !== 'follow'"
          :model-value="!!board.chatbox"
          label="Also say it in the chatbox"
          @update:model-value="board.chatbox = !!$event"
        />
      </template>
    </template>

    <UFormField
      v-else-if="leashStep"
      label="Leash moves me"
    >
      <SegmentedControl
        v-model="leashOn"
        :items="[
          { value: 'on', label: 'Turn it on' },
          { value: 'off', label: 'Turn it off' }
        ]"
        class="w-fit"
        accent
        aria-label="Leash moves me"
      />
    </UFormField>

    <BhapticsStepFields
      v-else-if="bhapticsStep"
      :model-value="bhapticsStep"
    />

    <OwoStepFields
      v-else-if="owoStep"
      :model-value="owoStep"
    />

    <UFormField
      v-if="stepCanWait(step) && (index > 0 || step.delayMs > 0)"
      label="How long after the start?"
    >
      <SegmentedControl
        v-model="delay"
        :items="delayItems"
        class="w-fit max-w-full flex-wrap"
        aria-label="How long after the start?"
      />
    </UFormField>
  </FormSection>
</template>
