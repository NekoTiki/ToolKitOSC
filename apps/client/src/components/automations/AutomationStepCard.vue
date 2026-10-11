<script setup lang="ts">
import SegmentedControl from '@renderer/components/ui/SegmentedControl.vue'
import { useAvatarDetails } from '@renderer/composables/useAvatarDetails'
import { useBoards } from '@renderer/composables/useBoards'
import { useChatbox } from '@renderer/composables/useChatbox'
import { useControls } from '@renderer/composables/useControls'
import { useLockedControls } from '@renderer/composables/useLockedControls'
import type { AutomationStep, ParameterValueType } from '@renderer/lib/tauri-bridge'
import { isNoisyAddress } from '@renderer/utils/addressFilter'
import type { ConditionType, ControlRunType, OutputActionType, ParameterActionType } from '@renderer/utils/automations'
import {
  actionsFor,
  CHATBOX_MAX,
  CONTROL_RUNS,
  controlRunsFor,
  defaultControlRun,
  defaultOutput,
  defaultParameterAction,
  formatMs,
  MAX_DELAY_MS,
  MAX_TOY_PULSE_MS,
  newStep,
  OUTPUT_ACTIONS,
  PARAMETER_ACTIONS,
  parameterActionsFor,
  parameterName,
  STEP_KINDS,
  stepCanWait,
  stepKindsFor
} from '@renderer/utils/automations'
import { CONTROL_TYPE_LABELS } from '@toolkitosc/shared-ui'
import { computed, ref } from 'vue'

// One step of an automation's "Do": what it drives, how, and how long after the trigger.
const props = defineProps<{
  index: number
  condition: ConditionType
  // The trigger's address: a step can't set it, or it would set itself off.
  trigger: string
  // Editing an automation for an avatar that isn't worn: its parameters can't be listed.
  otherAvatar: boolean
  canRemove: boolean
}>()
const emit = defineEmits<{ remove: [] }>()
const step = defineModel<AutomationStep>({ required: true })

const { avatarDetails } = useAvatarDetails()
const { boards } = useBoards()
const showAllParameters = ref(false)

const { controls } = useControls()
const { lockedControlGroups } = useLockedControls()
const { enabled: chatboxEnabled } = useChatbox()

// The kinds that can run on this condition, plus the current one so a step that no longer fits
// still shows what it is.
const kinds = computed(() => {
  const fitting = stepKindsFor(props.condition)
  return fitting.includes(step.value.kind) ? fitting : [...fitting, step.value.kind]
})

const kindItems = computed(() => kinds.value.map((kind) => ({ value: kind, label: STEP_KINDS[kind].label, icon: STEP_KINDS[kind].icon })))

const setKind = (kind: AutomationStep['kind']): void => {
  if (step.value.kind === kind) return
  const next = newStep(kind, props.condition)
  step.value = { ...next, delayMs: stepCanWait(next) ? step.value.delayMs : 0 }
}

// USlider can hand back an array; only plain numbers are taken (see OpenShockFields.vue).
const fromSlider = (value: number | number[] | undefined, apply: (value: number) => void): void => {
  if (typeof value === 'number') apply(value)
}

const toMs = (value: number | string, max = Infinity): number => Math.min(max, Math.max(0, Math.round(Number(value))))

// ---- ESP32 output

const board = computed(() => (step.value.kind === 'board-output' ? step.value : null))

const targetItems = computed(() =>
  boards.value.flatMap((b) =>
    (b.config ?? []).map((output) => ({
      label: `${output.label} · ${b.name}${b.status === 'online' ? '' : ' (offline)'}`,
      value: `${b.id}:${output.pin}`
    }))
  )
)

// The picked output as saved on its board, for its name and max on-time.
const targetOutput = computed(() =>
  board.value ? boards.value.find((b) => b.id === board.value!.boardId)?.config?.find((o) => o.pin === board.value!.pin) : undefined
)

const targetPwm = computed(
  () => !!board.value && !!boards.value.find((b) => b.id === board.value!.boardId)?.outputs.some((p) => p.pin === board.value!.pin && p.pwm)
)

const fitPulse = (): void => {
  const output = board.value?.output
  if (output?.type === 'pulse' && targetOutput.value && output.onMs > targetOutput.value.maxOnMs) {
    output.onMs = targetOutput.value.maxOnMs
  }
}

const target = computed({
  get: () => (board.value && board.value.pin >= 0 ? `${board.value.boardId}:${board.value.pin}` : undefined),
  set: (value: string | undefined) => {
    if (!board.value || !value) return
    const [boardId, pin] = value.split(':')
    board.value.boardId = boardId!
    board.value.pin = Number(pin)
    fitPulse()
  }
})

// A pulse can't outlast the output's max on-time: the board would cut it short.
const pulseTooLong = computed(
  () => board.value?.output.type === 'pulse' && !!targetOutput.value && board.value.output.onMs > targetOutput.value.maxOnMs
)

const outputType = computed({
  get: (): OutputActionType => board.value?.output.type ?? 'pulse',
  set: (type: OutputActionType) => {
    if (!board.value) return
    board.value.output = defaultOutput(type)
    if (!stepCanWait(board.value)) board.value.delayMs = 0
    fitPulse()
  }
})

const outputItems = computed(() =>
  actionsFor(props.condition)
    .filter((type) => !OUTPUT_ACTIONS[type].needsPwm || targetPwm.value || !target.value)
    .map((type) => ({ value: type, label: OUTPUT_ACTIONS[type].label }))
)

// ---- Avatar parameter

const param = computed(() => (step.value.kind === 'avatar-parameter' ? step.value : null))

// Only parameters VRChat accepts input for, minus the trigger.
const parameterItems = computed(() =>
  (avatarDetails.value?.parameters ?? [])
    .filter((p) => p.input && p.input.address !== props.trigger && (showAllParameters.value || !isNoisyAddress(p.name)))
    .filter((p) => parameterActionsFor(props.condition, p.input!.type as ParameterValueType).length)
    .map((p) => ({ label: `${p.name} · ${p.input!.type}`, value: p.input!.address, valueType: p.input!.type as ParameterValueType }))
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

const parameterType = computed({
  get: (): ParameterActionType => param.value?.set.type ?? 'set',
  set: (type: ParameterActionType) => {
    if (!param.value) return
    param.value.set = defaultParameterAction(type, param.value.valueType)
    if (!stepCanWait(param.value)) param.value.delayMs = 0
  }
})

const parameterTypeItems = computed(() =>
  param.value
    ? parameterActionsFor(props.condition, param.value.valueType).map((type) => ({ value: type, label: PARAMETER_ACTIONS[type].label }))
    : []
)

const parameterValue = computed({
  get: () => (param.value && 'value' in param.value.set ? param.value.set.value : 0),
  set: (value: number) => {
    if (param.value && 'value' in param.value.set) param.value.set.value = value
  }
})

const boolValue = computed({
  get: (): 'on' | 'off' => (parameterValue.value >= 0.5 ? 'on' : 'off'),
  set: (value: 'on' | 'off') => (parameterValue.value = value === 'on' ? 1 : 0)
})

// ---- Control

const controlStep = computed(() => (step.value.kind === 'control' ? step.value : null))

const allControls = computed(() => controls.value.flatMap((group) => group.controls.map((control) => ({ group, control }))))

const pickedControl = computed(() =>
  controlStep.value
    ? allControls.value.find((c) => c.group.id === controlStep.value!.groupId && c.control.id === controlStep.value!.controlId)?.control
    : undefined
)

// Only controls with a way to run on this condition.
const controlItems = computed(() =>
  allControls.value
    .filter(({ control }) => controlRunsFor(props.condition, control.type).length)
    .map(({ group, control }) => ({
      label: `${control.name} · ${CONTROL_TYPE_LABELS[control.type]} · ${group.name}`,
      value: `${group.id}:${control.id}`
    }))
)

const controlTarget = computed({
  get: () => (controlStep.value?.controlId ? `${controlStep.value.groupId}:${controlStep.value.controlId}` : undefined),
  set: (value: string | undefined) => {
    if (!controlStep.value || !value) return
    const [groupId, controlId] = value.split(':')
    const control = allControls.value.find((c) => c.group.id === groupId && c.control.id === controlId)?.control
    if (!control) return
    controlStep.value.groupId = groupId!
    controlStep.value.controlId = controlId!
    controlStep.value.controlType = control.type
    controlStep.value.run = defaultControlRun(controlRunsFor(props.condition, control.type)[0]!, control)
  }
})

const controlRunType = computed({
  get: (): ControlRunType => controlStep.value?.run.type ?? 'press',
  set: (type: ControlRunType) => {
    if (!controlStep.value) return
    controlStep.value.run = defaultControlRun(type, pickedControl.value)
    if (!stepCanWait(controlStep.value)) controlStep.value.delayMs = 0
  }
})

const controlRunItems = computed(() =>
  controlStep.value
    ? controlRunsFor(props.condition, controlStep.value.controlType).map((type) => ({ value: type, label: CONTROL_RUNS[type].label }))
    : []
)

const controlRunDescription = computed(() =>
  controlStep.value?.controlType === 'open-shock-shocker'
    ? "Uses the shocker's own intensity, duration and cooldown."
    : CONTROL_RUNS[controlRunType.value].description
)

// What a Set or Hold value looks like for this control.
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

const controlValue = computed(() => {
  const run = controlStep.value?.run
  return run && (run.type === 'set' || run.type === 'hold') ? run.value : undefined
})

const setControlValue = (value: boolean | number | string): void => {
  const run = controlStep.value?.run
  if (run && (run.type === 'set' || run.type === 'hold')) run.value = value
}

const controlBool = computed({
  get: (): 'on' | 'off' => (controlValue.value ? 'on' : 'off'),
  set: (value: 'on' | 'off') => setControlValue(value === 'on')
})

// Enum options and Toggle Logic inputs go by index; patterns by id.
const choiceItems = computed((): { label: string; value: string }[] => {
  const control = pickedControl.value
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

const controlChoice = computed({
  get: () => String(controlValue.value ?? ''),
  set: (value: string) => setControlValue(controlStep.value?.controlType === 'intiface-pattern' ? value : Number(value))
})

// ---- Profile

const profileStep = computed(() => (step.value.kind === 'profile' ? step.value : null))

// USelect can't hold null, so No profile goes by a stand-in.
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

// ---- Chatbox

const chatboxStep = computed(() => (step.value.kind === 'chatbox' ? step.value : null))

const toInt = (value: number | string): number => Math.min(255, Math.max(0, Math.round(Number(value) || 0)))
</script>

<template>
  <div class="grid gap-3.5 rounded-field border border-default bg-(--aurora-glass) p-3.5">
    <!-- The header carries the delay, so the body is only about what the step does. -->
    <div class="flex flex-wrap items-center gap-2.5">
      <span class="text-sm font-semibold text-highlighted">Step {{ index + 1 }}</span>
      <label
        v-if="stepCanWait(step)"
        class="ml-auto flex items-center gap-2 text-sm text-muted"
        :title="`How long after it's triggered this step starts. Up to ${formatMs(MAX_DELAY_MS)}.`"
      >
        After
        <UInput
          :model-value="step.delayMs"
          type="number"
          min="0"
          :max="MAX_DELAY_MS"
          step="100"
          class="w-28"
          aria-label="Milliseconds after it's triggered"
          @update:model-value="step.delayMs = toMs($event, MAX_DELAY_MS)"
        />
        ms
      </label>
      <UButton
        v-if="canRemove"
        icon="i-lucide-trash-2"
        color="neutral"
        variant="ghost"
        :class="stepCanWait(step) ? '' : 'ml-auto'"
        :aria-label="`Remove step ${index + 1}`"
        @click="emit('remove')"
      />
    </div>

    <div class="grid gap-1.5">
      <SegmentedControl
        :model-value="step.kind"
        :items="kindItems"
        aria-label="What this step does"
        class="w-fit max-w-full flex-wrap"
        @update:model-value="setKind"
      />
      <p class="text-[13px] text-muted">
        {{ STEP_KINDS[step.kind].description }}
      </p>
    </div>

    <template v-if="board">
      <UFormField
        label="Output"
        :description="targetItems.length ? undefined : 'No outputs yet. Add them in Settings › ESP32 boards › Edit.'"
      >
        <USelectMenu
          v-model="target"
          :items="targetItems"
          value-key="value"
          placeholder="Pick an output"
          class="w-full"
        />
      </UFormField>

      <UFormField
        label="Action"
        :description="OUTPUT_ACTIONS[outputType].description"
      >
        <SegmentedControl
          v-model="outputType"
          :items="outputItems"
          aria-label="Action"
          class="w-fit max-w-full flex-wrap"
        />
      </UFormField>

      <div
        v-if="board.output.type === 'pulse'"
        class="grid gap-3 sm:grid-cols-3"
      >
        <UFormField
          label="On for"
          :description="targetOutput ? `Milliseconds. Up to ${formatMs(targetOutput.maxOnMs)}, ${targetOutput.label}'s max on-time.` : 'Milliseconds.'"
          :error="pulseTooLong ? `Longer than ${formatMs(targetOutput!.maxOnMs)}. Shorten it, or raise the max on-time in Settings › ESP32 boards.` : false"
        >
          <UInput
            :model-value="board.output.onMs"
            type="number"
            min="1"
            :max="targetOutput?.maxOnMs"
            step="100"
            class="w-full"
            @update:model-value="board.output.type === 'pulse' && (board.output.onMs = toMs($event))"
          />
        </UFormField>
        <UFormField
          label="Times"
          description="1 to 100."
        >
          <UInput
            v-model.number="board.output.count"
            type="number"
            min="1"
            max="100"
            step="1"
            class="w-full"
          />
        </UFormField>
        <UFormField
          v-if="board.output.count > 1"
          label="Gap"
          description="Milliseconds between pulses."
        >
          <UInput
            :model-value="board.output.offMs"
            type="number"
            min="0"
            step="100"
            class="w-full"
            @update:model-value="board.output.type === 'pulse' && (board.output.offMs = toMs($event))"
          />
        </UFormField>
      </div>

      <div
        v-else-if="board.output.type === 'follow'"
        class="grid gap-3 sm:grid-cols-2"
      >
        <UFormField :label="`At 0: ${Math.round(board.output.min * 100)} %`">
          <USlider
            :model-value="board.output.min"
            :min="0"
            :max="1"
            :step="0.01"
            @update:model-value="fromSlider($event, (value) => board?.output.type === 'follow' && (board.output.min = value))"
          />
        </UFormField>
        <UFormField :label="`At 1: ${Math.round(board.output.max * 100)} %`">
          <USlider
            :model-value="board.output.max"
            :min="0"
            :max="1"
            :step="0.01"
            @update:model-value="fromSlider($event, (value) => board?.output.type === 'follow' && (board.output.max = value))"
          />
        </UFormField>
      </div>
    </template>

    <template v-else-if="param">
      <UFormField
        v-if="otherAvatar"
        label="Parameter"
        description="Wear this automation's avatar to pick another parameter."
      >
        <UInput
          :model-value="param.parameter ? `${parameterName(param.parameter)} · ${param.valueType}` : 'None picked'"
          class="w-full"
          disabled
        />
      </UFormField>
      <UFormField
        v-else
        label="Parameter"
      >
        <div class="grid gap-2">
          <USelectMenu
            v-model="parameter"
            :items="parameterItems"
            value-key="value"
            placeholder="Pick a parameter"
            class="w-full"
            virtualize
          />
          <UCheckbox
            v-model="showAllParameters"
            label="Show built-in parameters too"
          />
        </div>
      </UFormField>

      <template v-if="param.parameter">
        <UFormField
          label="Action"
          :description="PARAMETER_ACTIONS[parameterType].description"
        >
          <SegmentedControl
            v-model="parameterType"
            :items="parameterTypeItems"
            aria-label="Action"
            class="w-fit max-w-full flex-wrap"
          />
        </UFormField>

        <UFormField
          v-if="param.set.type === 'set' || param.set.type === 'hold'"
          :label="param.set.type === 'hold' ? 'Value while held' : 'Value'"
        >
          <SegmentedControl
            v-if="param.valueType === 'Bool'"
            v-model="boolValue"
            class="w-fit"
            :items="[
              { value: 'on', label: 'On' },
              { value: 'off', label: 'Off' }
            ]"
            aria-label="Value"
          />
          <div
            v-else-if="param.valueType === 'Int'"
            class="max-w-40"
          >
            <UInput
              :model-value="parameterValue"
              type="number"
              min="0"
              max="255"
              step="1"
              class="w-full"
              @update:model-value="parameterValue = toInt($event)"
            />
          </div>
          <div
            v-else
            class="grid gap-1.5"
          >
            <span class="text-sm text-muted tabular-nums">{{ parameterValue.toFixed(2) }}</span>
            <USlider
              :model-value="parameterValue"
              :min="-1"
              :max="1"
              :step="0.01"
              @update:model-value="fromSlider($event, (value) => (parameterValue = value))"
            />
          </div>
        </UFormField>

        <div
          v-else-if="param.set.type === 'follow'"
          class="grid gap-3 sm:grid-cols-2"
        >
          <UFormField :label="`At 0: ${param.set.min.toFixed(2)}`">
            <USlider
              :model-value="param.set.min"
              :min="-1"
              :max="1"
              :step="0.01"
              @update:model-value="fromSlider($event, (value) => param?.set.type === 'follow' && (param.set.min = value))"
            />
          </UFormField>
          <UFormField :label="`At 1: ${param.set.max.toFixed(2)}`">
            <USlider
              :model-value="param.set.max"
              :min="-1"
              :max="1"
              :step="0.01"
              @update:model-value="fromSlider($event, (value) => param?.set.type === 'follow' && (param.set.max = value))"
            />
          </UFormField>
        </div>
      </template>
    </template>

    <template v-else-if="controlStep">
      <UFormField
        v-if="otherAvatar"
        label="Control"
        description="Wear this automation's avatar to pick another control."
      >
        <UInput
          :model-value="controlStep.controlId ? 'A control on that avatar' : 'None picked'"
          class="w-full"
          disabled
        />
      </UFormField>
      <UFormField
        v-else
        label="Control"
        :description="controlItems.length ? undefined : 'None of your controls can run on this condition.'"
      >
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
          label="Action"
          :description="controlRunDescription"
        >
          <SegmentedControl
            v-model="controlRunType"
            :items="controlRunItems"
            aria-label="Action"
            class="w-fit max-w-full flex-wrap"
          />
        </UFormField>

        <UFormField
          v-if="controlStep.run.type === 'set' || controlStep.run.type === 'hold'"
          :label="controlStep.run.type === 'hold' ? 'While held' : 'Value'"
        >
          <SegmentedControl
            v-if="valueKind === 'bool'"
            v-model="controlBool"
            class="w-fit"
            :items="[
              { value: 'on', label: 'On' },
              { value: 'off', label: 'Off' }
            ]"
            aria-label="Value"
          />
          <USelect
            v-else-if="valueKind === 'choice'"
            v-model="controlChoice"
            :items="choiceItems"
            class="w-full max-w-80"
          />
          <div
            v-else
            class="grid gap-1.5"
          >
            <span class="text-sm text-muted tabular-nums">{{ Math.round(Number(controlStep.run.value) * 100) }} %</span>
            <USlider
              :model-value="Number(controlStep.run.value)"
              :min="0"
              :max="1"
              :step="0.01"
              @update:model-value="fromSlider($event, (value) => setControlValue(value))"
            />
          </div>
        </UFormField>

        <div
          v-else-if="controlStep.run.type === 'pulse'"
          class="grid gap-3 sm:grid-cols-2"
        >
          <UFormField :label="`Intensity: ${Math.round(controlStep.run.value * 100)} %`">
            <USlider
              :model-value="controlStep.run.value"
              :min="0"
              :max="1"
              :step="0.01"
              @update:model-value="fromSlider($event, (value) => controlStep?.run.type === 'pulse' && (controlStep.run.value = value))"
            />
          </UFormField>
          <UFormField
            label="For"
            :description="`Milliseconds. Up to ${formatMs(MAX_TOY_PULSE_MS)}.`"
          >
            <UInput
              :model-value="controlStep.run.ms"
              type="number"
              min="1"
              :max="MAX_TOY_PULSE_MS"
              step="100"
              class="w-full"
              @update:model-value="controlStep.run.type === 'pulse' && (controlStep.run.ms = toMs($event, MAX_TOY_PULSE_MS))"
            />
          </UFormField>
        </div>

        <div
          v-else-if="controlStep.run.type === 'follow'"
          class="grid gap-3 sm:grid-cols-2"
        >
          <UFormField :label="`At 0: ${Math.round(controlStep.run.min * 100)} %`">
            <USlider
              :model-value="controlStep.run.min"
              :min="0"
              :max="1"
              :step="0.01"
              @update:model-value="fromSlider($event, (value) => controlStep?.run.type === 'follow' && (controlStep.run.min = value))"
            />
          </UFormField>
          <UFormField :label="`At 1: ${Math.round(controlStep.run.max * 100)} %`">
            <USlider
              :model-value="controlStep.run.max"
              :min="0"
              :max="1"
              :step="0.01"
              @update:model-value="fromSlider($event, (value) => controlStep?.run.type === 'follow' && (controlStep.run.max = value))"
            />
          </UFormField>
        </div>
      </template>
    </template>

    <template v-else-if="profileStep">
      <UFormField
        label="Profile"
        :description="profileStep.whileHeld ? 'Active while the condition is true, then back to the profile you had.' : undefined"
      >
        <UInput
          v-if="otherAvatar"
          model-value="A profile on that avatar"
          class="w-full"
          disabled
        />
        <USelect
          v-else
          v-model="profileTarget"
          :items="profileItems"
          class="w-full max-w-80"
        />
      </UFormField>
    </template>

    <template v-else-if="chatboxStep">
      <UFormField
        label="Message"
        :hint="`${chatboxStep.text.length} / ${CHATBOX_MAX}`"
        :description="chatboxEnabled ? 'Sent at most every 3 seconds.' : undefined"
        :error="chatboxEnabled ? false : 'The chatbox is off, so this won\'t post anything. Turn it on in Settings › VRChat chatbox.'"
      >
        <UInput
          v-model="chatboxStep.text"
          :maxlength="CHATBOX_MAX"
          placeholder="Boop!"
          class="w-full"
          autocomplete="off"
        />
      </UFormField>
    </template>

    <p
      v-else-if="step.kind === 'stop-everything'"
      class="text-sm text-muted"
    >
      Stops every toy, pattern, shock and board output and pauses your controls, like the button in the top bar.
    </p>
  </div>
</template>
