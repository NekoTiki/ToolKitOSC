<script setup lang="ts">
import ConditionExtraFields from '@renderer/components/automations/ConditionExtraFields.vue'
import ParameterListen from '@renderer/components/parameters/ParameterListen.vue'
import ChoiceCard from '@renderer/components/ui/ChoiceCard.vue'
import FormSection from '@renderer/components/ui/FormSection.vue'
import SegmentedControl from '@renderer/components/ui/SegmentedControl.vue'
import { useAddons } from '@renderer/composables/useAddons'
import { useAutomations } from '@renderer/composables/useAutomations'
import { useAvatarDetails } from '@renderer/composables/useAvatarDetails'
import { useBoards } from '@renderer/composables/useBoards'
import { useControls } from '@renderer/composables/useControls'
import { useOscMessages } from '@renderer/composables/useOscMessages'
import type { Automation, AutomationTrigger, LeashEvent, ParameterValueType, ToyEvent } from '@renderer/lib/tauri-bridge'
import { isNoisyAddress } from '@renderer/utils/addressFilter'
import type { AppEventType, ConditionType } from '@renderer/utils/automations'
import {
  CONDITIONS,
  defaultCondition,
  fitStepsToTrigger,
  formatMinutes,
  newAutomation,
  parameterFamily,
  parameterName,
  setTriggerParameter,
  STOPPED_STEP,
  TOY_EVENTS
} from '@renderer/utils/automations'
import { EASY_HELD, seconds } from '@renderer/utils/automationSentence'
import type { Hand } from '@renderer/utils/builtinParameters'
import { GESTURES, gestureTrigger, HAND_ADDRESS, handOf, YOU_STATES, youState, youTrigger } from '@renderer/utils/builtinParameters'
import { computed, ref } from 'vue'

// Easy mode, "what starts it": what kind of thing, then which one and how it should react, in
// plain words. Each choice maps onto the same trigger the Advanced editor edits.
const props = defineProps<{ otherAvatar: boolean }>()
const automation = defineModel<Automation>({ required: true })

const { avatarDetails } = useAvatarDetails()
const { describeContext, automations } = useAutomations()
const { controls } = useControls()
const { isOn, offAddonForAutomationKind, offAddonForControlType } = useAddons()
const { args } = useOscMessages()
const { boards } = useBoards()

type Choice = 'avatar' | 'gesture' | 'you' | 'viewer' | 'switch' | 'leash' | 'toys' | 'board' | 'shock' | 'touch' | 'chain' | 'every' | 'resume' | 'stopped'

const CHOICES: { value: Choice; kind: AutomationTrigger['kind']; title: string; description: string; icon: string }[] = [
  { value: 'avatar', kind: 'avatar-parameter', title: 'Something on your avatar', description: 'A boop, a headpat, a grab, a toggle in your menu.', icon: 'i-lucide-person-standing' },
  { value: 'gesture', kind: 'avatar-parameter', title: 'A hand gesture', description: 'A fist, a point, a thumbs up, on either hand.', icon: 'i-lucide-hand' },
  { value: 'you', kind: 'avatar-parameter', title: 'What you do', description: 'You go AFK, sit down, mute yourself, get in a station.', icon: 'i-lucide-user-round' },
  { value: 'viewer', kind: 'app-event', title: 'A viewer does something', description: 'Joins your share page, leaves, or uses a control.', icon: 'i-lucide-users' },
  { value: 'switch', kind: 'avatar-changed', title: 'You put on an avatar', description: 'Also when the app starts while you wear it.', icon: 'i-lucide-repeat' },
  { value: 'leash', kind: 'leash', title: 'Your leash', description: 'Someone grabs it, pulls you to running or lets go.', icon: 'i-tkosc-collar' },
  { value: 'toys', kind: 'toy-event', title: 'Your toys', description: 'A toy connects, disconnects or runs low.', icon: 'i-lucide-vibrate' },
  { value: 'board', kind: 'board-event', title: 'An ESP32 board', description: 'It goes offline or comes back.', icon: 'i-lucide-cpu' },
  { value: 'shock', kind: 'shock-sent', title: 'A shock goes out', description: 'When OpenShock actually sends it.', icon: 'i-lucide-zap' },
  { value: 'touch', kind: 'touch', title: 'Avatar touch', description: 'Touch starts or stops driving your toys or gear.', icon: 'i-lucide-hand' },
  { value: 'chain', kind: 'after-automation', title: 'Another automation', description: 'Runs right after another one starts or ends.', icon: 'i-lucide-link' },
  { value: 'every', kind: 'every', title: 'Every so often', description: 'A reminder every few minutes while the app is open.', icon: 'i-lucide-timer' },
  { value: 'resume', kind: 'app-event', title: 'Controls come back', description: 'You turn Stop everything off again.', icon: 'i-lucide-power' },
  { value: 'stopped', kind: 'app-event', title: 'Stop everything turns on', description: 'Only the chatbox can still say something.', icon: 'i-lucide-octagon-x' }
]

const when = computed(() => automation.value.when)

const choice = computed((): Choice => {
  const w = when.value
  if (w.kind === 'avatar-parameter') return handOf(w.parameter) ? 'gesture' : youState(w.parameter) ? 'you' : 'avatar'
  if (w.kind === 'avatar-changed') return 'switch'
  if (w.kind === 'leash') return 'leash'
  if (w.kind === 'every') return 'every'
  if (w.kind === 'toy-event') return 'toys'
  if (w.kind === 'board-event') return 'board'
  if (w.kind === 'shock-sent') return 'shock'
  if (w.kind === 'touch') return 'touch'
  if (w.kind === 'after-automation') return 'chain'
  return w.event.type === 'resumed' ? 'resume' : w.event.type === 'stopped' ? 'stopped' : 'viewer'
})

// An add-on's kind shows while it's on, or while this automation already uses it.
const choices = computed(() => CHOICES.filter((c) => !offAddonForAutomationKind(c.kind) || c.value === choice.value))

const choose = (value: Choice): void => {
  if (value === choice.value || !avatarDetails.value) return
  const { id, name } = avatarDetails.value
  automation.value.when =
    value === 'avatar'
      ? newAutomation(id, name).when
      : value === 'gesture'
        ? gestureTrigger(id, name, 'right', 1)
        : value === 'you'
          ? youTrigger(id, name, YOU_STATES[0]!)
          : value === 'switch'
            ? { kind: 'avatar-changed', avatarId: id, avatarName: name }
            : value === 'leash'
              ? { kind: 'leash', event: 'grabbed' }
              : value === 'every'
                ? { kind: 'every', minutes: 30 }
                : value === 'toys'
                  ? { kind: 'toy-event', event: 'battery-low' }
                  : value === 'board'
                    ? { kind: 'board-event', event: 'offline', boardId: null }
                    : value === 'shock'
                      ? { kind: 'shock-sent' }
                      : value === 'touch'
                        ? { kind: 'touch', event: 'starts' }
                        : value === 'chain'
                          ? { kind: 'after-automation', automationId: '', whenStops: false }
                : { kind: 'app-event', event: value === 'resume' ? { type: 'resumed' } : value === 'stopped' ? { type: 'stopped' } : { type: 'viewer-joined' } }
  // Everything else is stopped by then: only the chatbox can still say so.
  if (value === 'stopped') automation.value.then = [{ ...STOPPED_STEP }]
  fitStepsToTrigger(automation.value)
}

const everyWhen = computed(() => (when.value.kind === 'every' ? when.value : null))
const toyWhen = computed(() => (when.value.kind === 'toy-event' ? when.value : null))
const TOY_ITEMS = (Object.keys(TOY_EVENTS) as ToyEvent[]).map((event) => ({ value: event, label: TOY_EVENTS[event] }))
const boardWhen = computed(() => (when.value.kind === 'board-event' ? when.value : null))
const boardItems = computed(() => [{ value: 'any', label: 'Any board' }, ...boards.value.map((b) => ({ value: b.id, label: b.name }))])
const touchWhen = computed(() => (when.value.kind === 'touch' ? when.value : null))
const chainWhen = computed(() => (when.value.kind === 'after-automation' ? when.value : null))
const chainItems = computed(() => automations.value.filter((a) => a.id !== automation.value.id).map((a) => ({ value: a.id, label: a.name })))
const EVERY_ITEMS = [5, 15, 30, 60, 120].map((minutes) => ({ value: String(minutes), label: formatMinutes(minutes) }))

// ---- Something on your avatar

const paramWhen = computed(() => (when.value.kind === 'avatar-parameter' && choice.value === 'avatar' ? when.value : null))
const search = ref('')

const parameters = computed(() =>
  (avatarDetails.value?.parameters ?? [])
    .filter((p) => p.output && !isNoisyAddress(p.name))
    .map((p) => {
      const address = p.output!.address
      return {
        address,
        valueType: p.output!.type as ParameterValueType,
        label: describeContext.value.controlForParameter?.(address)?.name ?? p.name,
        family: parameterFamily(address)
      }
    })
    .sort((a, b) => a.label.localeCompare(b.label))
)

const shown = computed(() => {
  const query = search.value.trim().toLowerCase()
  return query ? parameters.value.filter((p) => `${p.label} ${parameterName(p.address)}`.toLowerCase().includes(query)) : parameters.value
})

const KIND_HINT: Record<ParameterValueType, string> = { Bool: 'On or off', Int: 'One of a few', Float: '0 to 100%' }

const now = (address: string, valueType: ParameterValueType): string | null => {
  const value = args.value[address]?.[0]
  if (value === undefined) return null
  if (valueType === 'Bool') return value ? 'on' : 'off'
  return valueType === 'Float' && typeof value === 'number' ? `${Math.round(value * 100)}%` : String(value)
}

const pick = (address: string): void => {
  const item = parameters.value.find((p) => p.address === address)
  setTriggerParameter(automation.value, address, item?.valueType ?? 'Bool')
}

// How it should react, by type, in plain words.
const HOW: Record<ParameterValueType, { value: ConditionType; title: string; description: string }[]> = {
  Bool: [
    { value: 'turns-on', title: 'The moment it turns on', description: 'A boop lands, a grab starts, you flip it on.' },
    { value: 'turns-off', title: 'When it turns off', description: 'They let go, you flip it off.' },
    { value: 'is-on', title: "For as long as it's on", description: 'What happens can last until it turns off.' },
    { value: 'taps', title: 'A few quick taps', description: 'A double boop, three pats in a row.' }
  ],
  Int: [
    { value: 'becomes', title: 'The moment it switches', description: 'Runs once each time it changes to this.' },
    { value: 'is', title: 'For as long as it stays', description: 'What happens can last until it changes again.' }
  ],
  Float: [
    { value: 'any', title: 'Follow how strong it is', description: 'A closer headpat buzzes harder.' },
    { value: 'rises-above', title: 'When it goes past a point', description: 'Runs once each time it crosses it.' },
    { value: 'is-above', title: "For as long as it's past a point", description: 'What happens lasts while it stays there.' },
    { value: 'falls-below', title: 'When it drops under a point', description: 'Runs once each time it goes back down.' },
    { value: 'is-between', title: 'While it stays in a range', description: 'Not too little, not too much.' },
    { value: 'stops-changing', title: 'When it settles', description: "Runs once it hasn't moved for a moment." }
  ]
}

const setHow = (type: ConditionType): void => {
  if (!paramWhen.value) return
  const value = 'value' in paramWhen.value.condition ? paramWhen.value.condition.value : undefined
  paramWhen.value.condition = defaultCondition(type)
  // The point or the option stays when switching between a moment and a state.
  if (value !== undefined && 'value' in paramWhen.value.condition) paramWhen.value.condition.value = value
  if (CONDITIONS[type].moment || type === 'any') paramWhen.value.heldMs = 0
  // More parameters only start moments.
  if (!CONDITIONS[type].moment) paramWhen.value.also = undefined
  fitStepsToTrigger(automation.value)
}

const conditionValue = computed({
  get: () => (paramWhen.value && 'value' in paramWhen.value.condition ? paramWhen.value.condition.value : 0),
  set: (value: number) => {
    if (paramWhen.value && 'value' in paramWhen.value.condition) paramWhen.value.condition.value = value
  }
})

// An Int's options, named by the control that drives it when there is one.
const intOptions = computed(() => {
  const control = paramWhen.value ? describeContext.value.controlForParameter?.(paramWhen.value.parameter) : undefined
  return control?.type === 'enum' || control?.type === 'step-enum' ? control.options.map((o) => ({ value: String(o.value), label: o.name })) : null
})

const isState = computed(() => !!paramWhen.value && !CONDITIONS[paramWhen.value.condition.type].moment && paramWhen.value.condition.type !== 'any')

const held = computed({
  get: () => String(paramWhen.value?.heldMs ?? 0),
  set: (value: string) => {
    if (paramWhen.value) paramWhen.value.heldMs = Number(value)
  }
})
const heldItems = EASY_HELD.map((ms) => ({ value: String(ms), label: ms ? seconds(ms) : 'At once' }))

// USlider can hand back an array; only plain numbers are taken (see OpenShockFields.vue).
const fromSlider = (value: number | number[] | undefined): void => {
  if (typeof value === 'number') conditionValue.value = value
}

// ---- A hand gesture, what you do: VRChat's own parameters

const builtinWhen = computed(() => (when.value.kind === 'avatar-parameter' && choice.value !== 'avatar' ? when.value : null))

const hand = computed({
  get: (): Hand => (builtinWhen.value && handOf(builtinWhen.value.parameter)) || 'right',
  set: (value: Hand) => {
    if (builtinWhen.value) builtinWhen.value.parameter = HAND_ADDRESS[value]
  }
})
const gesture = computed({
  get: () => String(builtinWhen.value && 'value' in builtinWhen.value.condition ? builtinWhen.value.condition.value : 1),
  set: (value: string) => {
    if (builtinWhen.value && 'value' in builtinWhen.value.condition) builtinWhen.value.condition.value = Number(value)
  }
})
// Neutral is left out: it's what every other gesture goes back to.
const gestureItems = GESTURES.map((label, value) => ({ value: String(value), label })).slice(1)

const youAddress = computed({
  get: () => builtinWhen.value?.parameter ?? YOU_STATES[0]!.address,
  set: (address: string) => {
    if (builtinWhen.value) builtinWhen.value.parameter = address
  }
})
const youItems = YOU_STATES.map((s) => ({ value: s.address, label: s.label }))

const builtinHow = computed((): { value: ConditionType; title: string; description: string }[] => {
  if (choice.value === 'gesture') {
    return [
      { value: 'becomes', title: 'The moment you make it', description: 'Runs once each time.' },
      { value: 'is', title: 'For as long as you hold it', description: 'What happens can last until you let go.' }
    ]
  }
  const state = builtinWhen.value ? youState(builtinWhen.value.parameter) : undefined
  return state
    ? [
        { value: 'turns-on', title: `You ${state.on}`, description: 'Runs once, the moment it happens.' },
        { value: 'turns-off', title: `You ${state.off}`, description: 'Runs once, the moment it happens.' },
        { value: 'is-on', title: `While you're ${state.is}`, description: 'What happens can last until it ends.' }
      ]
    : []
})

const setBuiltinHow = (type: ConditionType): void => {
  if (!builtinWhen.value) return
  const value = 'value' in builtinWhen.value.condition ? builtinWhen.value.condition.value : undefined
  builtinWhen.value.condition = defaultCondition(type)
  // The gesture stays when switching between a moment and holding it.
  if (value !== undefined && 'value' in builtinWhen.value.condition) builtinWhen.value.condition.value = value
  if (CONDITIONS[type].moment) builtinWhen.value.heldMs = 0
  fitStepsToTrigger(automation.value)
}

const builtinHeld = computed({
  get: () => String(builtinWhen.value?.heldMs ?? 0),
  set: (value: string) => {
    if (builtinWhen.value) builtinWhen.value.heldMs = Number(value)
  }
})

// ---- A viewer does something

const eventWhen = computed(() => (when.value.kind === 'app-event' && choice.value === 'viewer' ? when.value : null))

const VIEWER_EVENTS: { value: Exclude<AppEventType, 'resumed'>; label: string }[] = [
  { value: 'viewer-joined', label: 'Joins' },
  { value: 'first-viewer', label: 'Is the first to join' },
  { value: 'last-viewer-left', label: 'Is the last to leave' },
  { value: 'viewer-command', label: 'Uses a control' },
  { value: 'viewer-held-back', label: 'Is held back' }
]

const eventType = computed({
  get: () => (eventWhen.value?.event.type ?? 'viewer-joined') as Exclude<AppEventType, 'resumed'>,
  set: (type) => {
    if (eventWhen.value) eventWhen.value.event = type === 'viewer-command' ? { type, controlId: null, controlType: null } : { type }
  }
})

// Which viewer commands count: any, any shock, or one control.
const SHOCK = 'open-shock-shocker'
const eventControl = computed({
  get: (): string => {
    const event = eventWhen.value?.event
    if (event?.type !== 'viewer-command') return 'any'
    return event.controlId ?? (event.controlType === SHOCK ? 'shock' : 'any')
  },
  set: (value: string) => {
    if (eventWhen.value?.event.type !== 'viewer-command') return
    eventWhen.value.event = {
      type: 'viewer-command',
      controlId: value === 'any' || value === 'shock' ? null : value,
      controlType: value === 'shock' ? SHOCK : null
    }
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

// ---- You put on an avatar

const changedWhen = computed(() => (when.value.kind === 'avatar-changed' ? when.value : null))

const changedItems = computed(() => {
  const items: { value: string; label: string }[] = []
  const w = changedWhen.value
  if (w?.avatarId && w.avatarId !== avatarDetails.value?.id) items.push({ value: w.avatarId, label: w.avatarName ?? 'That avatar' })
  if (avatarDetails.value) items.push({ value: avatarDetails.value.id, label: `${avatarDetails.value.name} (this one)` })
  items.push({ value: 'any', label: 'Any avatar' })
  return items
})

const changedTarget = computed({
  get: () => changedWhen.value?.avatarId ?? 'any',
  set: (value: string) => {
    if (!changedWhen.value) return
    if (value === 'any') Object.assign(changedWhen.value, { avatarId: null, avatarName: null })
    else if (value === avatarDetails.value?.id) Object.assign(changedWhen.value, { avatarId: value, avatarName: avatarDetails.value.name })
  }
})

// ---- Your leash

const leashWhen = computed(() => (when.value.kind === 'leash' ? when.value : null))
const LEASH_ITEMS: { value: LeashEvent; label: string }[] = [
  { value: 'grabbed', label: 'Grabbed' },
  { value: 'running', label: 'Pulled to running' },
  { value: 'released', label: 'Let go' }
]
</script>

<template>
  <FormSection title="What starts it?">
    <div class="grid grid-cols-[repeat(auto-fill,minmax(13rem,1fr))] gap-2.5">
      <ChoiceCard
        v-for="c in choices"
        :key="c.value"
        :title="c.title"
        :description="c.description"
        :icon="c.icon"
        :selected="choice === c.value"
        @click="choose(c.value)"
      />
    </div>

    <template v-if="paramWhen">
      <p
        v-if="props.otherAvatar"
        class="text-sm text-muted"
      >
        For {{ paramWhen.avatarName }}. Wear that avatar to pick something else.
      </p>
      <template v-else>
        <ParameterListen
          :can-pick="(address) => parameters.some((p) => p.address === address)"
          @pick="pick"
        />
        <UFormField label="Or pick it from your avatar">
          <div class="grid gap-2.5">
            <UInput
              v-model="search"
              icon="i-lucide-search"
              placeholder="Search"
              class="w-full"
            />
            <div class="grid max-h-80 grid-cols-[repeat(auto-fill,minmax(14rem,1fr))] gap-2 overflow-y-auto pr-1">
              <button
                v-for="p in shown"
                :key="p.address"
                type="button"
                class="grid min-h-14 cursor-pointer grid-cols-[minmax(0,1fr)_auto] items-center gap-x-2.5 rounded-field border bg-(--aurora-well) px-3.5 py-2 text-left"
                :class="paramWhen.parameter === p.address ? 'border-primary bg-primary/12' : 'border-transparent'"
                :aria-pressed="paramWhen.parameter === p.address"
                @click="pick(p.address)"
              >
                <b class="truncate font-semibold text-highlighted">{{ p.label }}</b>
                <span class="row-span-2 font-mono text-xs text-muted">{{ now(p.address, p.valueType) }}</span>
                <small class="truncate text-xs text-muted">{{ KIND_HINT[p.valueType] }}{{ p.family ? ` · ${p.family}` : '' }}</small>
              </button>
            </div>
          </div>
        </UFormField>
      </template>

      <template v-if="paramWhen.parameter">
        <UFormField label="When should it react?">
          <div class="grid grid-cols-[repeat(auto-fill,minmax(13rem,1fr))] gap-2.5">
            <ChoiceCard
              v-for="how in HOW[paramWhen.valueType]"
              :key="how.value"
              :title="how.title"
              :description="how.description"
              :selected="paramWhen.condition.type === how.value"
              @click="setHow(how.value)"
            />
          </div>
        </UFormField>

        <UFormField
          v-if="'value' in paramWhen.condition && paramWhen.valueType === 'Int'"
          label="Which one?"
        >
          <SegmentedControl
            v-if="intOptions"
            :model-value="String(conditionValue)"
            :items="intOptions"
            class="w-fit max-w-full flex-wrap"
            accent
            aria-label="Which one?"
            @update:model-value="conditionValue = Number($event)"
          />
          <UInput
            v-else
            v-model.number="conditionValue"
            type="number"
            step="1"
            class="max-w-40"
          />
        </UFormField>
        <UFormField
          v-else-if="'value' in paramWhen.condition"
          :label="`The point: ${Math.round(conditionValue * 100)}%`"
        >
          <USlider
            :model-value="conditionValue"
            :min="0"
            :max="1"
            :step="0.01"
            @update:model-value="fromSlider"
          />
        </UFormField>

        <ConditionExtraFields v-model="paramWhen.condition" />

        <UFormField
          v-if="isState"
          label="Ignore quick brushes"
          description="How long it has to last before anything happens."
        >
          <SegmentedControl
            v-model="held"
            :items="heldItems"
            class="w-fit max-w-full flex-wrap"
            aria-label="Ignore quick brushes"
          />
        </UFormField>
      </template>
    </template>

    <template v-else-if="builtinWhen">
      <template v-if="choice === 'gesture'">
        <UFormField label="Which hand?">
          <SegmentedControl
            v-model="hand"
            :items="[
              { value: 'left', label: 'Left' },
              { value: 'right', label: 'Right' }
            ]"
            class="w-fit"
            accent
            aria-label="Which hand?"
          />
        </UFormField>
        <UFormField label="Which gesture?">
          <SegmentedControl
            v-model="gesture"
            :items="gestureItems"
            class="w-fit max-w-full flex-wrap"
            accent
            aria-label="Which gesture?"
          />
        </UFormField>
      </template>
      <UFormField
        v-else
        label="What?"
      >
        <SegmentedControl
          v-model="youAddress"
          :items="youItems"
          class="w-fit max-w-full flex-wrap"
          accent
          aria-label="What you do"
        />
      </UFormField>
      <UFormField label="When should it react?">
        <div class="grid grid-cols-[repeat(auto-fill,minmax(13rem,1fr))] gap-2.5">
          <ChoiceCard
            v-for="how in builtinHow"
            :key="how.value"
            :title="how.title"
            :description="how.description"
            :selected="builtinWhen.condition.type === how.value"
            @click="setBuiltinHow(how.value)"
          />
        </div>
      </UFormField>
      <UFormField
        v-if="!CONDITIONS[builtinWhen.condition.type].moment"
        label="Ignore quick ones"
        description="How long it has to last before anything happens."
      >
        <SegmentedControl
          v-model="builtinHeld"
          :items="heldItems"
          class="w-fit max-w-full flex-wrap"
          aria-label="Ignore quick ones"
        />
      </UFormField>
    </template>

    <UFormField
      v-else-if="toyWhen"
      label="When…"
    >
      <SegmentedControl
        v-model="toyWhen.event"
        :items="TOY_ITEMS"
        class="w-fit max-w-full flex-wrap"
        accent
        aria-label="Toy event"
      />
    </UFormField>

    <template v-else-if="boardWhen">
      <UFormField label="Which board?">
        <SegmentedControl
          :model-value="boardWhen.boardId ?? 'any'"
          :items="boardItems"
          class="w-fit max-w-full flex-wrap"
          accent
          aria-label="Which board?"
          @update:model-value="boardWhen.boardId = $event === 'any' ? null : $event"
        />
      </UFormField>
      <UFormField label="When it…">
        <SegmentedControl
          v-model="boardWhen.event"
          :items="[
            { value: 'offline', label: 'Goes offline' },
            { value: 'online', label: 'Comes back' }
          ]"
          class="w-fit"
          accent
          aria-label="Board event"
        />
      </UFormField>
    </template>

    <template v-else-if="chainWhen">
      <UFormField label="Which automation?">
        <USelectMenu
          v-model="chainWhen.automationId"
          :items="chainItems"
          value-key="value"
          placeholder="Pick an automation"
          class="w-full"
        />
      </UFormField>
      <UFormField
        label="When it…"
        description="Up to 3 automations in a row can start each other from one event."
      >
        <SegmentedControl
          :model-value="chainWhen.whenStops ? 'stops' : 'runs'"
          :items="[
            { value: 'runs', label: 'Runs' },
            { value: 'stops', label: 'Is done' }
          ]"
          class="w-fit"
          accent
          aria-label="When it runs or ends"
          @update:model-value="chainWhen.whenStops = $event === 'stops'"
        />
      </UFormField>
    </template>

    <UFormField
      v-else-if="touchWhen"
      label="When touch…"
      description="Any zone or socket that drives your toys, bHaptics or OWO gear."
    >
      <SegmentedControl
        v-model="touchWhen.event"
        :items="[
          { value: 'starts', label: 'Starts' },
          { value: 'stops', label: 'Stops' }
        ]"
        class="w-fit"
        accent
        aria-label="Touch"
      />
    </UFormField>

    <UFormField
      v-else-if="everyWhen"
      label="How often?"
      description="Counted from when the app starts or you save it."
    >
      <SegmentedControl
        :model-value="String(everyWhen.minutes)"
        :items="EVERY_ITEMS"
        class="w-fit max-w-full flex-wrap"
        accent
        aria-label="How often?"
        @update:model-value="everyWhen.minutes = Number($event)"
      />
    </UFormField>

    <template v-else-if="eventWhen">
      <UFormField label="The viewer…">
        <SegmentedControl
          v-model="eventType"
          :items="VIEWER_EVENTS"
          class="w-fit max-w-full flex-wrap"
          accent
          aria-label="What the viewer does"
        />
      </UFormField>
      <UFormField
        v-if="eventType === 'viewer-command'"
        label="Which control?"
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
      v-else-if="changedWhen"
      label="Which avatar?"
    >
      <SegmentedControl
        v-model="changedTarget"
        :items="changedItems"
        class="w-fit max-w-full flex-wrap"
        accent
        aria-label="Which avatar?"
      />
    </UFormField>

    <UFormField
      v-else-if="leashWhen"
      label="When your leash is…"
    >
      <SegmentedControl
        v-model="leashWhen.event"
        :items="LEASH_ITEMS"
        class="w-fit max-w-full flex-wrap"
        accent
        aria-label="Leash"
      />
    </UFormField>
  </FormSection>
</template>
