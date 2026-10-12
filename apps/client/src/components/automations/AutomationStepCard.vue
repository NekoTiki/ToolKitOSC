<script setup lang="ts">
import ChoiceCard from '@renderer/components/ui/ChoiceCard.vue'
import SegmentedControl from '@renderer/components/ui/SegmentedControl.vue'
import { useAvatarDetails } from '@renderer/composables/useAvatarDetails'
import { useBoards } from '@renderer/composables/useBoards'
import type { AutomationStep, ParameterValueType } from '@renderer/lib/tauri-bridge'
import { isNoisyAddress } from '@renderer/utils/addressFilter'
import type { ConditionType, OutputActionType, ParameterActionType } from '@renderer/utils/automations'
import {
  actionsFor,
  defaultOutput,
  defaultParameterAction,
  formatMs,
  MAX_DELAY_MS,
  newBoardStep,
  newParameterStep,
  OUTPUT_ACTIONS,
  PARAMETER_ACTIONS,
  parameterActionsFor,
  parameterName,
  stepCanWait
} from '@renderer/utils/automations'
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

const setKind = (kind: AutomationStep['kind']): void => {
  if (step.value.kind === kind) return
  const next = kind === 'board-output' ? newBoardStep(props.condition) : newParameterStep(props.condition)
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

const toInt =(value: number | string): number => Math.min(255, Math.max(0, Math.round(Number(value) || 0)))
</script>

<template>
  <div class="grid gap-3.5 rounded-field border border-default bg-(--aurora-glass) p-3.5">
    <div class="flex items-center gap-2">
      <span class="text-sm font-semibold text-highlighted">Step {{ index + 1 }}</span>
      <UButton
        v-if="canRemove"
        icon="i-lucide-x"
        color="neutral"
        variant="ghost"
        size="sm"
        class="ml-auto"
        :aria-label="`Remove step ${index + 1}`"
        @click="emit('remove')"
      />
    </div>

    <div class="grid gap-2.5 sm:grid-cols-2">
      <ChoiceCard
        title="ESP32 output"
        description="Switch something wired to one of your boards."
        icon="i-lucide-cpu"
        :selected="step.kind === 'board-output'"
        @click="setKind('board-output')"
      />
      <ChoiceCard
        title="Avatar parameter"
        description="Set one of your avatar's own parameters."
        icon="i-lucide-sliders-horizontal"
        :selected="step.kind === 'avatar-parameter'"
        @click="setKind('avatar-parameter')"
      />
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

    <UFormField
      v-if="stepCanWait(step)"
      label="After"
      :description="`Milliseconds after it's triggered. Up to ${formatMs(MAX_DELAY_MS)}.`"
      class="max-w-56"
    >
      <UInput
        :model-value="step.delayMs"
        type="number"
        min="0"
        :max="MAX_DELAY_MS"
        step="100"
        class="w-full"
        @update:model-value="step.delayMs = toMs($event, MAX_DELAY_MS)"
      />
    </UFormField>
  </div>
</template>
