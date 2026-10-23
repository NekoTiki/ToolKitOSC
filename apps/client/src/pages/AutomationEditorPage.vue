<script setup lang="ts">
import AutomationGuardRow from '@renderer/components/automations/AutomationGuardRow.vue'
import AutomationStepCard from '@renderer/components/automations/AutomationStepCard.vue'
import AutomationTimeline from '@renderer/components/automations/AutomationTimeline.vue'
import AutomationEasyEditor from '@renderer/components/automations/easy/AutomationEasyEditor.vue'
import DraftNotice from '@renderer/components/DraftNotice.vue'
import ParameterListen from '@renderer/components/parameters/ParameterListen.vue'
import FormSection from '@renderer/components/ui/FormSection.vue'
import OptionGrid from '@renderer/components/ui/OptionGrid.vue'
import SegmentedControl from '@renderer/components/ui/SegmentedControl.vue'
import { useAddons } from '@renderer/composables/useAddons'
import { useAreYouSureModal } from '@renderer/composables/useAreYouSureModal'
import { useAutomationIdeas } from '@renderer/composables/useAutomationIdeas'
import { useAutomations } from '@renderer/composables/useAutomations'
import { useAvatarDetails } from '@renderer/composables/useAvatarDetails'
import { useBoards } from '@renderer/composables/useBoards'
import { useControls } from '@renderer/composables/useControls'
import { useOscMessages } from '@renderer/composables/useOscMessages'
import { useStopEverything } from '@renderer/composables/useStopEverything'
import type { Automation, AutomationStep, AutomationTrigger, LeashEvent, ParameterValueType } from '@renderer/lib/tauri-bridge'
import { isNoisyAddress } from '@renderer/utils/addressFilter'
import type { AppEventType, ConditionType } from '@renderer/utils/automations'
import {
  APP_EVENTS,
  conditionHolds,
  CONDITIONS,
  conditionsFor,
  defaultCondition,
  describe,
  fitStepsToTrigger,
  formatMs,
  LEASH_EVENTS,
  MAX_EVERY_MINUTES,
  MAX_GUARDS,
  MAX_HELD_MS,
  MAX_STEPS,
  newAutomation,
  newStep,
  parameterFamily,
  parameterName,
  setTriggerParameter,
  STEP_KINDS,
  stepFits,
  stepKindsFor,
  STOPPED_STEP,
  triggerCondition} from '@renderer/utils/automations'
import { useLocalStorage } from '@vueuse/core'
import { computed, ref, toRaw, watch } from 'vue'
import { onBeforeRouteLeave, useRoute, useRouter } from 'vue-router'

// Create or edit one automation: when (an avatar parameter changes), do one or more things (on a
// board output or another avatar parameter), each after its own delay. Both sides are typed by
// `kind`, so other kinds of "when" and "do" get their own cards here later.
const route = useRoute()
const router = useRouter()
const toast = useToast()
const { automations, running, save, remove, test, describeContext } = useAutomations()
const { controls, getControl } = useControls()
const { avatarDetails } = useAvatarDetails()
const { boards } = useBoards()
const { paused } = useStopEverything()
const { openModal: confirm } = useAreYouSureModal()
const { isOn, offAddonForAutomationKind, offAddonForControlType } = useAddons()

const isNew = route.name === 'automation-new'
const automationId = isNew ? null : String(route.params.automationId)
const stored = computed(() => automations.value.find((a) => a.id === automationId))

const draft = ref<Automation | null>(null)
// What the form started from, to tell what changed.
const savedJson = ref('')
const leaving = ref(false)
const saving = ref(false)
const showAllParameters = ref(false)
// Easy (the sentence) or Advanced (every setting), remembered on this computer. An idea always
// opens in Easy, without changing what's remembered until you switch.
const savedMode = useLocalStorage<'easy' | 'advanced'>('automations_editorMode', 'easy')
const ideaVisit = ref(!!route.query.idea)
const mode = computed({
  get: () => (ideaVisit.value ? 'easy' : savedMode.value),
  set: (value: 'easy' | 'advanced') => {
    ideaVisit.value = false
    savedMode.value = value
  }
})
const { ideas } = useAutomationIdeas()

// Fills the form once what it needs has loaded (the list and the avatar arrive asynchronously).
watch(
  [() => stored.value?.id, () => avatarDetails.value?.id],
  () => {
    if (draft.value) return
    if (isNew && avatarDetails.value) {
      // From "Start from an idea", or blank.
      const idea = ideas.value.find((i) => i.id === route.query.idea)
      draft.value = idea ? structuredClone(toRaw(idea.automation)) : newAutomation(avatarDetails.value.id, avatarDetails.value.name)
      savedJson.value = JSON.stringify(draft.value)
    } else if (stored.value) {
      draft.value = structuredClone(toRaw(stored.value))
      savedJson.value = JSON.stringify(stored.value)
    }
  },
  { immediate: true }
)

const dirty = computed(() => !!draft.value && JSON.stringify(draft.value) !== savedJson.value)

const paramWhen = computed(() => (draft.value?.when.kind === 'avatar-parameter' ? draft.value.when : null))
const changedWhen = computed(() => (draft.value?.when.kind === 'avatar-changed' ? draft.value.when : null))
const eventWhen = computed(() => (draft.value?.when.kind === 'app-event' ? draft.value.when : null))

const eventType = computed({
  get: (): AppEventType => eventWhen.value?.event.type ?? 'viewer-command',
  set: (type: AppEventType) => {
    if (eventWhen.value) eventWhen.value.event = type === 'viewer-command' ? { type, controlId: null, controlType: null } : { type }
    // Everything else is stopped by then: only the chatbox can still say so.
    if (type === 'stopped' && draft.value) draft.value.then = [{ ...STOPPED_STEP }]
  }
})

const everyWhen = computed(() => (draft.value?.when.kind === 'every' ? draft.value.when : null))

const eventItems = (Object.keys(APP_EVENTS) as AppEventType[]).map((type) => ({ value: type, label: APP_EVENTS[type].label }))

const leashWhen = computed(() => (draft.value?.when.kind === 'leash' ? draft.value.when : null))
const leashEventItems = (Object.keys(LEASH_EVENTS) as LeashEvent[]).map((event) => ({ value: event, label: LEASH_EVENTS[event].label }))

// Which viewer commands count: any, any shock, or one control (by id; the worn avatar's controls).
const SHOCK = 'open-shock-shocker'
const eventControl = computed({
  get: (): string => {
    const event = eventWhen.value?.event
    if (event?.type !== 'viewer-command') return 'any'
    return event.controlId ?? (event.controlType === SHOCK ? 'shock' : 'any')
  },
  set: (value: string) => {
    if (eventWhen.value?.event.type !== 'viewer-command') return
    eventWhen.value.event =
      value === 'any'
        ? { type: 'viewer-command', controlId: null, controlType: null }
        : value === 'shock'
          ? { type: 'viewer-command', controlId: null, controlType: SHOCK }
          : { type: 'viewer-command', controlId: value, controlType: null }
  }
})

const eventControlItems = computed(() => [
  { label: 'Any control', value: 'any' },
  ...(isOn('openshock') || eventControl.value === 'shock' ? [{ label: 'Any shock', value: 'shock' }] : []),
  ...controls.value.flatMap((group) =>
    group.controls
      .filter((control) => control.id === eventControl.value || !offAddonForControlType(control.type))
      .map((control) => ({ label: `${control.name} · ${group.name}`, value: control.id }))
  )
])

// ---- Only if

const addGuard = (): void => {
  if (!draft.value || draft.value.onlyIf.length >= MAX_GUARDS) return
  draft.value.onlyIf.push({ kind: 'parameter', parameter: '', valueType: 'Bool', condition: { type: 'is-on' }, not: false })
}

// An automation for another avatar keeps its parameter, controls and profiles: this avatar's
// lists don't have them. One for any avatar uses the worn avatar's.
const otherAvatar = computed(() => {
  const when = draft.value?.when
  const avatarId = when && 'avatarId' in when ? when.avatarId : null
  return !!avatarId && avatarId !== avatarDetails.value?.id
})

const setTriggerKind = (kind: AutomationTrigger['kind']): void => {
  if (!draft.value || draft.value.when.kind === kind || !avatarDetails.value) return
  const { id, name } = avatarDetails.value
  draft.value.when =
    kind === 'avatar-changed'
      ? { kind, avatarId: id, avatarName: name }
      : kind === 'app-event'
        ? { kind, event: { type: 'viewer-command', controlId: null, controlType: null } }
        : kind === 'leash'
          ? { kind, event: 'grabbed' }
          : kind === 'every'
            ? { kind, minutes: 30 }
            : newAutomation(id, name).when
  fitSteps()
}

const TRIGGER_KINDS: Record<AutomationTrigger['kind'], { label: string; description: string; icon: string }> = {
  'avatar-parameter': { label: 'Avatar parameter', description: "One of your avatar's parameters changes.", icon: 'i-lucide-person-standing' },
  'avatar-changed': { label: 'Avatar changed', description: 'You put on an avatar.', icon: 'i-lucide-repeat' },
  'app-event': {
    label: 'Share page',
    description: 'A viewer uses a control, joins or leaves, or controls resume.',
    icon: 'i-lucide-users'
  },
  leash: {
    label: 'Leash',
    description: 'Someone grabs your avatar leash, pulls you to running or lets go.',
    icon: 'i-tkosc-collar'
  },
  every: { label: 'Every so often', description: 'Every few minutes while the app is open.', icon: 'i-lucide-timer' }
}

// An add-on's kinds show while it's on, or while this automation already uses one.
const triggerKindItems = computed(() =>
  (Object.keys(TRIGGER_KINDS) as AutomationTrigger['kind'][])
    .filter((kind) => !offAddonForAutomationKind(kind) || kind === draft.value?.when.kind)
    .map((kind) => ({ value: kind, label: TRIGGER_KINDS[kind].label, icon: TRIGGER_KINDS[kind].icon }))
)

const triggerKind = computed({
  get: (): AutomationTrigger['kind'] => draft.value?.when.kind ?? 'avatar-parameter',
  set: setTriggerKind
})

// Avatar changed: the worn avatar (or the one it was made for), or any avatar.
const changedTarget = computed({
  get: (): string => changedWhen.value?.avatarId ?? 'any',
  set: (value: string) => {
    if (!changedWhen.value) return
    if (value === 'any') {
      changedWhen.value.avatarId = null
      changedWhen.value.avatarName = null
    } else if (avatarDetails.value && value === avatarDetails.value.id) {
      changedWhen.value.avatarId = avatarDetails.value.id
      changedWhen.value.avatarName = avatarDetails.value.name
    }
  }
})

const changedItems = computed(() => {
  const items: { value: string; label: string }[] = []
  const own = stored.value?.when.kind === 'avatar-changed' ? stored.value.when : null
  if (own?.avatarId && own.avatarId !== avatarDetails.value?.id) items.push({ value: own.avatarId, label: own.avatarName ?? 'That avatar' })
  if (avatarDetails.value) items.push({ value: avatarDetails.value.id, label: avatarDetails.value.name })
  items.push({ value: 'any', label: 'Any avatar' })
  return items
})

// ---- When

const parameterItems = computed(() =>
  (avatarDetails.value?.parameters ?? [])
    .filter((p) => p.output && (showAllParameters.value || !isNoisyAddress(p.name)))
    .map((p) => {
      const family = parameterFamily(p.output!.address)
      return { label: `${p.name} · ${p.output!.type}${family ? ` · ${family}` : ''}`, value: p.output!.address, valueType: p.output!.type }
    })
    .sort((a, b) => a.label.localeCompare(b.label))
)

const fitSteps = (): void => {
  if (draft.value) fitStepsToTrigger(draft.value)
}

const parameter = computed({
  get: () => paramWhen.value?.parameter || undefined,
  set: (address: string | undefined) => {
    if (!draft.value || !address) return
    const item = parameterItems.value.find((p) => p.value === address)
    setTriggerParameter(draft.value, address, (item?.valueType ?? 'Bool') as ParameterValueType)
  }
})

// For an avatar change, the moment it happens.
const conditionType = computed({
  get: (): ConditionType => (draft.value ? triggerCondition(draft.value.when) : 'turns-on'),
  set: (type: ConditionType) => {
    if (!paramWhen.value) return
    paramWhen.value.condition = defaultCondition(type)
    // Only a state waits to be held.
    if (CONDITIONS[type].moment || type === 'any') paramWhen.value.heldMs = 0
    fitSteps()
  }
})

const conditionItems = computed(() =>
  paramWhen.value ? conditionsFor(paramWhen.value.valueType).map((type) => ({ value: type, label: CONDITIONS[type].label })) : []
)

const conditionValue = computed({
  get: () => (paramWhen.value && 'value' in paramWhen.value.condition ? paramWhen.value.condition.value : 0),
  set: (value: number) => {
    if (paramWhen.value && 'value' in paramWhen.value.condition) paramWhen.value.condition.value = value
  }
})

// ---- Listen and live value

// Anything the avatar reports can be a trigger.
const canTrigger = (address: string): boolean => !!avatarDetails.value?.parameters.some((p) => p.output?.address === address)

const { args } = useOscMessages()

// The trigger parameter's value right now, as a number (Bools are 0/1).
const liveValue = computed((): number | null => {
  const address = paramWhen.value?.parameter
  const value = address ? args.value[address]?.[0] : undefined
  if (typeof value === 'boolean') return value ? 1 : 0
  return typeof value === 'number' ? value : null
})

const liveText = computed(() => {
  if (liveValue.value === null || !paramWhen.value) return null
  const shown = paramWhen.value.valueType === 'Bool' ? (liveValue.value ? 'on' : 'off') : String(liveValue.value)
  const holds = conditionHolds(paramWhen.value.condition, liveValue.value)
  return holds === null ? `Right now: ${shown}` : `Right now: ${shown}, so the condition is ${holds ? 'true' : 'not true'}`
})

const canHold = computed(() => !CONDITIONS[conditionType.value].moment && conditionType.value !== 'any')

// ---- Do

// A new step is the first kind that fits: an Avatar parameter, unless the trigger rules it out.
const addStep = (): void => {
  if (!draft.value || draft.value.then.length >= MAX_STEPS) return
  const condition = conditionType.value
  draft.value.then.push(newStep(stepKindsFor(condition, (kind) => !offAddonForAutomationKind(kind))[0]!, condition))
}

const removeStep = (index: number): void => {
  draft.value?.then.splice(index, 1)
}

// What's missing from one step, or null.
const stepProblem = (step: AutomationStep): string | null => {
  const condition = conditionType.value
  if (!stepFits(step, condition)) {
    return CONDITIONS[condition].moment
      ? `${STEP_KINDS[step.kind].label} can't hold. Change it, or pick a state like "is on".`
      : `${STEP_KINDS[step.kind].label} needs a moment, like "turns on".`
  }

  switch (step.kind) {
    case 'avatar-parameter':
      if (!step.parameter) return 'Pick the parameter each step sets.'
      return step.parameter === paramWhen.value?.parameter ? "A step can't set the parameter that triggers it." : null
    case 'control':
      if (!step.controlId) return 'Pick the control each step runs.'
      if (!otherAvatar.value && !getControl(step.groupId, step.controlId)) return 'One of its controls was deleted. Pick another.'
      return null
    case 'chatbox':
      return step.text.trim() ? null : 'Write the chatbox message.'
    case 'board-output': {
      if (step.pin < 0) return 'Pick the output each step drives.'
      if (step.output.type !== 'pulse') return null
      if (step.output.onMs <= 0) return 'Set how long the pulse lasts.'
      const output = boards.value.find((b) => b.id === step.boardId)?.config?.find((o) => o.pin === step.pin)
      return output && step.output.onMs > output.maxOnMs ? `The pulse is longer than ${output.label}'s max on-time.` : null
    }
    default:
      return null
  }
}

// Every ESP32 timing is entered in milliseconds.
const toMs = (value: number | string): number => Math.max(0, Math.round(Number(value)))

// USlider can hand back an array; only plain numbers are taken (see OpenShockFields.vue).
const fromSlider = (value: number | number[] | undefined, apply: (value: number) => void): void => {
  if (typeof value === 'number') apply(value)
}

// ---- Saving

const problem = computed((): string | null => {
  if (!draft.value) return null
  if (!draft.value.name.trim()) return 'Give it a name.'
  if (paramWhen.value && !paramWhen.value.parameter) return 'Pick the parameter that triggers it.'
  if (!draft.value.then.length) return 'Add something for it to do.'
  if (draft.value.onlyIf.some((g) => g.kind === 'parameter' && !g.parameter)) return 'Pick the parameter each "only if" checks.'
  for (const step of draft.value.then) {
    const missing = stepProblem(step)
    if (missing) return missing
  }
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
        :meta="mode === 'advanced' && draft && !problem ? describe(draft, describeContext) : undefined"
        :crumbs="[{ label: 'Automations', to: '/automations' }, { label: isNew ? 'New' : draft?.name || 'Untitled' }]"
      >
        <template #actions>
          <span
            v-if="automationId && running.has(automationId)"
            class="rounded-full bg-secondary/15 px-2.5 py-1 text-xs font-medium text-secondary"
          >Running</span>
          <SegmentedControl
            v-if="draft"
            v-model="mode"
            :items="[
              { value: 'easy', label: 'Easy', icon: 'i-lucide-text' },
              { value: 'advanced', label: 'Advanced', icon: 'i-lucide-sliders-horizontal' }
            ]"
            aria-label="Editor mode"
          />
        </template>
      </PageHeader>
      <DraftNotice
        v-if="stored?.createdBy"
        :created-by="stored.createdBy"
        @keep="submit"
        @discard="deleteAutomation"
      />
    </template>

    <div
      v-if="!draft"
      class="grid gap-3.5"
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

    <!-- One column, read from top to bottom. Easy: the sentence and the panel for the part picked.
         Advanced: when, only if, what it does, then its limits. -->
    <div
      v-else
      class="grid gap-3.5"
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

      <AutomationEasyEditor
        v-if="mode === 'easy'"
        v-model="draft"
        :other-avatar="otherAvatar"
        @advanced="mode = 'advanced'"
      />
      <template v-else>
        <FormSection
          title="When"
          :description="TRIGGER_KINDS[draft.when.kind].description"
        >
          <OptionGrid
            v-model="triggerKind"
            :items="triggerKindItems"
            aria-label="When"
          />

          <template v-if="eventWhen">
            <UFormField
              label="Event"
              :description="APP_EVENTS[eventType].description"
            >
              <OptionGrid
                v-model="eventType"
                min-width="12rem"
                :items="eventItems"
                aria-label="Event"
              />
            </UFormField>
            <UFormField
              v-if="eventWhen.event.type === 'viewer-command'"
              label="Control"
              description="Only when it goes through: locks, limits and cooldowns still apply to viewers."
            >
              <USelectMenu
                v-model="eventControl"
                :items="eventControlItems"
                value-key="value"
                class="w-full"
              />
            </UFormField>
          </template>

          <UFormField
            v-if="leashWhen"
            label="Event"
            :description="LEASH_EVENTS[leashWhen.event].description"
          >
            <OptionGrid
              v-model="leashWhen.event"
              min-width="12rem"
              :items="leashEventItems"
              aria-label="Leash event"
            />
          </UFormField>

          <UFormField
            v-if="everyWhen"
            label="Every"
            :description="`Minutes, up to ${MAX_EVERY_MINUTES} (a day). Counted from when the app starts or you save it.`"
          >
            <UInput
              :model-value="everyWhen.minutes"
              type="number"
              min="1"
              :max="MAX_EVERY_MINUTES"
              class="w-28"
              @update:model-value="everyWhen.minutes = Math.min(MAX_EVERY_MINUTES, Math.max(1, Math.round(Number($event) || 1)))"
            />
          </UFormField>

          <UFormField
            v-if="changedWhen"
            label="Avatar"
            description="Also runs when the app starts while you already wear it."
          >
            <OptionGrid
              v-model="changedTarget"
              :items="changedItems"
              aria-label="Avatar"
            />
          </UFormField>

          <template v-if="paramWhen">
            <UFormField
              v-if="otherAvatar"
              label="Parameter"
              :description="`For ${paramWhen.avatarName}. Wear that avatar to pick another parameter.`"
            >
              <UInput
                :model-value="`${parameterName(paramWhen.parameter)} · ${paramWhen.valueType}`"
                class="w-full"
                disabled
              />
            </UFormField>
            <UFormField
              v-else
              label="Parameter"
              :description="paramWhen.anyAvatar ? 'On any avatar that has a parameter with this name.' : `On ${paramWhen.avatarName}. It only runs while you wear this avatar.`"
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

            <ParameterListen
              v-if="!otherAvatar"
              :can-pick="canTrigger"
              @pick="parameter = $event"
            />

            <USwitch
              v-if="paramWhen.parameter"
              v-model="paramWhen.anyAvatar"
              label="Any avatar with this parameter"
              description="For names that are the same on all your avatars, like VRCFury Haptics (VFH/…), OSC Goes Brrr (OGB/…) or a PhysBone's _IsGrabbed."
            />

            <UFormField
              v-if="paramWhen.parameter"
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
                  v-if="CONDITIONS[conditionType].needsValue && paramWhen.valueType === 'Int'"
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

            <p
              v-if="liveText && !otherAvatar"
              class="flex items-center gap-1.5 text-sm text-muted"
            >
              <UIcon
                name="i-lucide-activity"
                class="size-4 shrink-0"
              />
              {{ liveText }}
            </p>

            <UFormField
              v-if="paramWhen.parameter && canHold"
              label="Held for"
              :description="`Milliseconds it has to stay true first, so a quick brush doesn't count. 0 for at once, up to ${formatMs(MAX_HELD_MS)}.`"
              class="max-w-72"
            >
              <UInput
                :model-value="paramWhen.heldMs"
                type="number"
                min="0"
                :max="MAX_HELD_MS"
                step="100"
                class="w-full"
                @update:model-value="paramWhen && (paramWhen.heldMs = Math.min(MAX_HELD_MS, toMs($event)))"
              />
            </UFormField>
          </template>
        </FormSection>

        <FormSection
          title="Only if"
          :description="
            draft.onlyIf.length
              ? draft.onlyIfAny
                ? 'One of these has to be true when it starts.'
                : 'All of these have to be true when it starts.'
              : 'Always runs. Add a condition to only run sometimes.'
          "
        >
          <SegmentedControl
            v-if="draft.onlyIf.length > 1"
            :model-value="draft.onlyIfAny ? 'any' : 'all'"
            :items="[
              { value: 'all', label: 'All of these' },
              { value: 'any', label: 'Any of these' }
            ]"
            aria-label="All or any"
            class="w-fit"
            @update:model-value="draft.onlyIfAny = $event === 'any' || undefined"
          />
          <template
            v-if="draft.onlyIf.length < MAX_GUARDS"
            #actions
          >
            <UButton
              icon="i-lucide-plus"
              color="neutral"
              variant="subtle"
              @click="addGuard"
            >
              Add
            </UButton>
          </template>
          <AutomationGuardRow
            v-for="(guard, index) in draft.onlyIf"
            :key="index"
            v-model="draft.onlyIf[index]!"
            :other-avatar="otherAvatar"
            @remove="draft.onlyIf.splice(index, 1)"
          />
        </FormSection>

        <FormSection
          title="Do"
          :description="`Its steps run together, each after its own delay. ${draft.then.length} of ${MAX_STEPS}.`"
        >
          <template
            v-if="draft.then.length < MAX_STEPS"
            #actions
          >
            <UButton
              icon="i-lucide-plus"
              @click="addStep"
            >
              Add a step
            </UButton>
          </template>
          <AutomationTimeline :steps="draft.then" />
          <div class="grid gap-3">
            <AutomationStepCard
              v-for="(step, index) in draft.then"
              :key="index"
              v-model="draft.then[index]!"
              :index="index"
              :condition="conditionType"
              :trigger="paramWhen?.parameter ?? ''"
              :other-avatar="otherAvatar"
              :can-remove="draft.then.length > 1"
              @remove="removeStep(index)"
            />
          </div>
        </FormSection>

        <FormSection
          v-if="CONDITIONS[conditionType].moment"
          title="Limits"
        >
          <div class="grid gap-3.5">
            <UFormField
              label="Cooldown"
              description="Milliseconds before it can run again. 0 for none."
            >
              <UInput
                :model-value="draft.cooldownMs"
                type="number"
                min="0"
                step="100"
                class="w-full"
                @update:model-value="draft.cooldownMs = toMs($event)"
              />
            </UFormField>
            <UFormField label="If it happens again while running">
              <SegmentedControl
                v-model="draft.retrigger"
                class="w-fit"
                :items="[
                  { value: 'restart', label: 'Start over' },
                  { value: 'ignore', label: 'Ignore it' }
                ]"
                aria-label="If it happens again while running"
              />
            </UFormField>
          </div>
        </FormSection>
      </template>
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
        {{ problem ?? testBlocked ?? 'Test runs every step once, with its delay, without waiting for the parameter.' }}
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
