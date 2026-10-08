<script setup lang="ts">
import ChoiceCard from '@renderer/components/ui/ChoiceCard.vue'
import FormSection from '@renderer/components/ui/FormSection.vue'
import SegmentedControl from '@renderer/components/ui/SegmentedControl.vue'
import { useAreYouSureModal } from '@renderer/composables/useAreYouSureModal'
import { useAutomations } from '@renderer/composables/useAutomations'
import { useAvatarDetails } from '@renderer/composables/useAvatarDetails'
import { useBoards } from '@renderer/composables/useBoards'
import { useStopEverything } from '@renderer/composables/useStopEverything'
import type { Automation, ParameterValueType } from '@renderer/lib/tauri-bridge'
import { isNoisyAddress } from '@renderer/utils/addressFilter'
import type { ConditionType, OutputActionType } from '@renderer/utils/automations'
import {
  actionsFor,
  CONDITIONS,
  conditionsFor,
  defaultCondition,
  defaultOutput,
  describe,
  newAutomation,
  OUTPUT_ACTIONS,
  parameterName
} from '@renderer/utils/automations'
import { computed, ref, toRaw, watch } from 'vue'
import { onBeforeRouteLeave, useRoute, useRouter } from 'vue-router'

// Create or edit one automation: when (an avatar parameter changes), do (something on a board
// output). Both sides are typed by `kind`, so other kinds of "when" and "do" get their own cards
// here later.
const route = useRoute()
const router = useRouter()
const toast = useToast()
const { automations, running, save, remove, test } = useAutomations()
const { avatarDetails } = useAvatarDetails()
const { boards } = useBoards()
const { paused } = useStopEverything()
const { openModal: confirm } = useAreYouSureModal()

const isNew = route.name === 'automation-new'
const automationId = isNew ? null : String(route.params.automationId)
const stored = computed(() => automations.value.find((a) => a.id === automationId))

const draft = ref<Automation | null>(null)
// What the form started from, to tell what changed.
const savedJson = ref('')
const leaving = ref(false)
const saving = ref(false)
const showAllParameters = ref(false)

// Fills the form once what it needs has loaded (the list and the avatar arrive asynchronously).
watch(
  [() => stored.value?.id, () => avatarDetails.value?.id],
  () => {
    if (draft.value) return
    if (isNew && avatarDetails.value) {
      draft.value = newAutomation(avatarDetails.value.id, avatarDetails.value.name)
      savedJson.value = JSON.stringify(draft.value)
    } else if (stored.value) {
      draft.value = structuredClone(toRaw(stored.value))
      savedJson.value = JSON.stringify(stored.value)
    }
  },
  { immediate: true }
)

const dirty = computed(() => !!draft.value && JSON.stringify(draft.value) !== savedJson.value)

// An automation for another avatar keeps its parameter: this avatar's list doesn't have it.
const otherAvatar = computed(() => !!draft.value && draft.value.when.avatarId !== avatarDetails.value?.id)

// ---- When

const parameterItems = computed(() =>
  (avatarDetails.value?.parameters ?? [])
    .filter((p) => p.output && (showAllParameters.value || !isNoisyAddress(p.name)))
    .map((p) => ({ label: `${p.name} · ${p.output!.type}`, value: p.output!.address, valueType: p.output!.type }))
    .sort((a, b) => a.label.localeCompare(b.label))
)

// Keeps the action valid when the condition changes: a hold can't follow a moment, and so on.
const fitAction = (): void => {
  if (!draft.value) return
  const condition = draft.value.when.condition.type
  if (!OUTPUT_ACTIONS[draft.value.then.output.type].fits(condition)) {
    draft.value.then.output = defaultOutput(actionsFor(condition)[0]!)
  }
}

const parameter = computed({
  get: () => draft.value?.when.parameter || undefined,
  set: (address: string | undefined) => {
    if (!draft.value || !address) return
    const item = parameterItems.value.find((p) => p.value === address)
    const valueType = (item?.valueType ?? 'Bool') as ParameterValueType

    draft.value.when.parameter = address
    draft.value.when.valueType = valueType
    if (CONDITIONS[draft.value.when.condition.type].valueType !== valueType) {
      draft.value.when.condition = defaultCondition(conditionsFor(valueType)[0]!)
    }
    if (!draft.value.name) draft.value.name = parameterName(address)
    fitAction()
  }
})

const conditionType = computed({
  get: (): ConditionType => draft.value?.when.condition.type ?? 'turns-on',
  set: (type: ConditionType) => {
    if (!draft.value) return
    draft.value.when.condition = defaultCondition(type)
    fitAction()
  }
})

const conditionItems = computed(() =>
  draft.value ? conditionsFor(draft.value.when.valueType).map((type) => ({ value: type, label: CONDITIONS[type].label })) : []
)

const conditionValue = computed({
  get: () => (draft.value && 'value' in draft.value.when.condition ? draft.value.when.condition.value : 0),
  set: (value: number) => {
    if (draft.value && 'value' in draft.value.when.condition) draft.value.when.condition.value = value
  }
})

// ---- Do

const targetItems = computed(() =>
  boards.value.flatMap((board) =>
    (board.config ?? []).map((output) => ({
      label: `${output.label} · ${board.name}${board.status === 'online' ? '' : ' (offline)'}`,
      value: `${board.id}:${output.pin}`
    }))
  )
)

const target = computed({
  get: () => (draft.value && draft.value.then.pin >= 0 ? `${draft.value.then.boardId}:${draft.value.then.pin}` : undefined),
  set: (value: string | undefined) => {
    if (!draft.value || !value) return
    const [boardId, pin] = value.split(':')
    draft.value.then.boardId = boardId!
    draft.value.then.pin = Number(pin)
    fitAction()
  }
})

const targetPwm = computed(() => {
  const then = draft.value?.then
  return !!then && !!boards.value.find((b) => b.id === then.boardId)?.outputs.some((p) => p.pin === then.pin && p.pwm)
})

const actionType = computed({
  get: (): OutputActionType => draft.value?.then.output.type ?? 'pulse',
  set: (type: OutputActionType) => {
    if (draft.value) draft.value.then.output = defaultOutput(type)
  }
})

const actionItems = computed(() =>
  actionsFor(conditionType.value)
    .filter((type) => !OUTPUT_ACTIONS[type].needsPwm || targetPwm.value || !target.value)
    .map((type) => ({ value: type, label: OUTPUT_ACTIONS[type].label }))
)

// USlider can hand back an array; only plain numbers are taken (see OpenShockFields.vue).
const fromSlider = (value: number | number[] | undefined, apply: (value: number) => void): void => {
  if (typeof value === 'number') apply(value)
}

// Seconds in the form, milliseconds in the data.
const seconds = (ms: number): number => Math.round(ms) / 1000
const toMs = (value: number | string): number => Math.max(0, Math.round(Number(value) * 1000))

// ---- Saving

const problem = computed((): string | null => {
  if (!draft.value) return null
  if (!draft.value.name.trim()) return 'Give it a name.'
  if (!draft.value.when.parameter) return 'Pick the parameter that triggers it.'
  if (!target.value) return 'Pick the output it drives.'
  return null
})

const submit = async (): Promise<void> => {
  if (!draft.value || problem.value || saving.value) return
  saving.value = true

  try {
    await save({ ...draft.value, name: draft.value.name.trim() })
    leaving.value = true
    toast.add({ title: `Saved "${draft.value.name.trim()}"`, icon: 'i-lucide-check', color: 'success' })
    void router.push('/automations')
  } catch (error) {
    toast.add({ title: "Couldn't save", description: String(error), icon: 'i-lucide-circle-x', color: 'error' })
  } finally {
    saving.value = false
  }
}

const testBlocked = computed((): string | null => {
  if (isNew || !stored.value) return 'Save it to test it.'
  if (dirty.value) return 'Save your changes to test them.'
  if (paused.value) return 'Stop everything is on.'
  return null
})

const runTest = async (): Promise<void> => {
  if (!automationId) return
  try {
    await test(automationId)
  } catch (error) {
    toast.add({ title: "Couldn't test it", description: String(error), icon: 'i-lucide-circle-x', color: 'error' })
  }
}

const deleteAutomation = async (): Promise<void> => {
  if (!draft.value || !automationId) return
  const ok = await confirm({
    title: 'Delete automation?',
    message: `Delete **${draft.value.name}**? This can't be undone.`,
    confirmText: 'Delete'
  })
  if (!ok) return

  await remove(automationId)
  leaving.value = true
  void router.push('/automations')
}

onBeforeRouteLeave(async () => {
  if (leaving.value || !dirty.value) return true
  return confirm({ title: 'Discard changes?', message: 'Your changes to this automation will be lost.', confirmText: 'Discard' })
})
</script>

<template>
  <PageLayout>
    <template #header>
      <PageHeader
        :title="isNew ? 'New automation' : draft?.name || 'Automation'"
        :meta="draft && !problem ? describe(draft, boards) : undefined"
        :crumbs="[{ label: 'Automations', to: '/automations' }, { label: isNew ? 'New' : draft?.name || 'Untitled' }]"
      >
        <template
          v-if="automationId && running.has(automationId)"
          #actions
        >
          <span class="rounded-full bg-secondary/15 px-2.5 py-1 text-xs font-medium text-secondary">Running</span>
        </template>
      </PageHeader>
    </template>

    <div
      v-if="!draft"
      class="grid max-w-3xl gap-3.5"
    >
      <FormSection
        :title="isNew ? 'Wear an avatar first' : 'This automation doesn\'t exist any more'"
        :description="isNew ? 'Automations start from one of your avatar\'s parameters. Start VRChat with OSC on.' : 'It may have been deleted.'"
      >
        <div>
          <UButton
            color="neutral"
            variant="subtle"
            to="/automations"
          >
            Back to Automations
          </UButton>
        </div>
      </FormSection>
    </div>

    <div
      v-else
      class="grid max-w-3xl gap-3.5"
    >
      <FormSection>
        <div class="flex flex-wrap items-end gap-3.5">
          <UFormField
            label="Name"
            class="min-w-60 flex-1"
          >
            <UInput
              v-model="draft.name"
              placeholder="Boop fan"
              class="w-full"
              autocomplete="off"
            />
          </UFormField>
          <USwitch
            v-model="draft.enabled"
            label="Enabled"
          />
        </div>
      </FormSection>

      <FormSection
        title="When"
        description="What starts it."
      >
        <div class="grid gap-2.5 sm:grid-cols-2">
          <ChoiceCard
            title="Avatar parameter"
            description="One of your avatar's parameters changes."
            icon="i-lucide-person-standing"
            :selected="draft.when.kind === 'avatar-parameter'"
          />
        </div>

        <template v-if="draft.when.kind === 'avatar-parameter'">
          <UFormField
            v-if="otherAvatar"
            label="Parameter"
            :description="`For ${draft.when.avatarName}. Wear that avatar to pick another parameter.`"
          >
            <UInput
              :model-value="`${parameterName(draft.when.parameter)} · ${draft.when.valueType}`"
              class="w-full"
              disabled
            />
          </UFormField>
          <UFormField
            v-else
            label="Parameter"
            :description="`On ${draft.when.avatarName}. It only runs while you wear this avatar.`"
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
                label="Show built-in parameters too (Velocity, GestureLeft…)"
              />
            </div>
          </UFormField>

          <UFormField
            v-if="draft.when.parameter"
            label="Condition"
          >
            <div class="grid gap-3">
              <SegmentedControl
                v-model="conditionType"
                :items="conditionItems"
                aria-label="Condition"
                class="w-fit max-w-full flex-wrap"
              />
              <div
                v-if="CONDITIONS[conditionType].needsValue && draft.when.valueType === 'Int'"
                class="max-w-40"
              >
                <UInput
                  v-model.number="conditionValue"
                  type="number"
                  step="1"
                  class="w-full"
                />
              </div>
              <div
                v-else-if="CONDITIONS[conditionType].needsValue"
                class="grid gap-1.5"
              >
                <span class="text-sm text-muted tabular-nums">{{ conditionValue.toFixed(2) }}</span>
                <USlider
                  :model-value="conditionValue"
                  :min="0"
                  :max="1"
                  :step="0.01"
                  @update:model-value="fromSlider($event, (value) => (conditionValue = value))"
                />
              </div>
            </div>
          </UFormField>
        </template>
      </FormSection>

      <FormSection
        title="Do"
        description="What happens."
      >
        <div class="grid gap-2.5 sm:grid-cols-2">
          <ChoiceCard
            title="ESP32 output"
            description="Switch something wired to one of your boards."
            icon="i-lucide-cpu"
            :selected="draft.then.kind === 'board-output'"
          />
        </div>

        <template v-if="draft.then.kind === 'board-output'">
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
            :description="OUTPUT_ACTIONS[actionType].description"
          >
            <SegmentedControl
              v-model="actionType"
              :items="actionItems"
              aria-label="Action"
              class="w-fit max-w-full flex-wrap"
            />
          </UFormField>

          <div
            v-if="draft.then.output.type === 'pulse'"
            class="grid gap-3 sm:grid-cols-3"
          >
            <UFormField
              label="On for"
              description="Seconds."
            >
              <UInput
                :model-value="seconds(draft.then.output.onMs)"
                type="number"
                min="0.05"
                step="0.1"
                class="w-full"
                @update:model-value="draft.then.output.type === 'pulse' && (draft.then.output.onMs = toMs($event))"
              />
            </UFormField>
            <UFormField
              label="Times"
              description="1 to 100."
            >
              <UInput
                v-model.number="draft.then.output.count"
                type="number"
                min="1"
                max="100"
                step="1"
                class="w-full"
              />
            </UFormField>
            <UFormField
              v-if="draft.then.output.count > 1"
              label="Gap"
              description="Seconds between pulses."
            >
              <UInput
                :model-value="seconds(draft.then.output.offMs)"
                type="number"
                min="0"
                step="0.1"
                class="w-full"
                @update:model-value="draft.then.output.type === 'pulse' && (draft.then.output.offMs = toMs($event))"
              />
            </UFormField>
          </div>

          <div
            v-else-if="draft.then.output.type === 'follow'"
            class="grid gap-3 sm:grid-cols-2"
          >
            <UFormField :label="`At 0: ${Math.round(draft.then.output.min * 100)} %`">
              <USlider
                :model-value="draft.then.output.min"
                :min="0"
                :max="1"
                :step="0.01"
                @update:model-value="fromSlider($event, (value) => draft?.then.output.type === 'follow' && (draft.then.output.min = value))"
              />
            </UFormField>
            <UFormField :label="`At 1: ${Math.round(draft.then.output.max * 100)} %`">
              <USlider
                :model-value="draft.then.output.max"
                :min="0"
                :max="1"
                :step="0.01"
                @update:model-value="fromSlider($event, (value) => draft?.then.output.type === 'follow' && (draft.then.output.max = value))"
              />
            </UFormField>
          </div>
        </template>
      </FormSection>

      <FormSection
        v-if="CONDITIONS[conditionType].moment"
        title="Limits"
      >
        <div class="flex flex-wrap items-start gap-3.5">
          <UFormField
            label="Cooldown"
            description="Seconds before it can run again. 0 for none."
            class="min-w-56 flex-1"
          >
            <UInput
              :model-value="seconds(draft.cooldownMs)"
              type="number"
              min="0"
              step="0.5"
              class="w-full"
              @update:model-value="draft.cooldownMs = toMs($event)"
            />
          </UFormField>
          <UFormField
            label="If it happens again while running"
            class="flex-1"
          >
            <SegmentedControl
              v-model="draft.retrigger"
              :items="[
                { value: 'restart', label: 'Start over' },
                { value: 'ignore', label: 'Ignore it' }
              ]"
              aria-label="If it happens again while running"
            />
          </UFormField>
        </div>
      </FormSection>
    </div>

    <template
      v-if="draft"
      #footer
    >
      <p class="mr-auto flex min-w-0 items-center gap-1.5 text-[12.5px] text-muted">
        <UIcon
          name="i-lucide-info"
          class="size-4 shrink-0"
        />
        {{ problem ?? testBlocked ?? 'Test runs the action once, without waiting for the parameter.' }}
      </p>
      <UButton
        v-if="!isNew"
        icon="i-lucide-trash-2"
        color="neutral"
        variant="subtle"
        @click="deleteAutomation"
      >
        Delete
      </UButton>
      <UButton
        v-if="!isNew"
        icon="i-lucide-zap"
        color="neutral"
        variant="subtle"
        :disabled="!!testBlocked"
        @click="runTest"
      >
        Test
      </UButton>
      <UButton
        icon="i-lucide-check"
        :loading="saving"
        :disabled="!!problem || (!isNew && !dirty)"
        @click="submit"
      >
        {{ isNew ? 'Create automation' : 'Save changes' }}
      </UButton>
    </template>
  </PageLayout>
</template>
