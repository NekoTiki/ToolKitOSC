<script setup lang="ts">
import EasyOftenPanel from '@renderer/components/automations/easy/EasyOftenPanel.vue'
import EasyOnlyPanel from '@renderer/components/automations/easy/EasyOnlyPanel.vue'
import EasySentence from '@renderer/components/automations/easy/EasySentence.vue'
import EasyStepPanel from '@renderer/components/automations/easy/EasyStepPanel.vue'
import EasyWhenPanel from '@renderer/components/automations/easy/EasyWhenPanel.vue'
import { useAddons } from '@renderer/composables/useAddons'
import { useAutomations } from '@renderer/composables/useAutomations'
import type { Automation } from '@renderer/lib/tauri-bridge'
import { MAX_STEPS, newStep, STEP_KINDS, stepCanWait, stepKindsFor, triggerCondition, triggerIcon } from '@renderer/utils/automations'
import { easyExtras, sentence, stepMissing } from '@renderer/utils/automationSentence'
import { computed, ref } from 'vue'

// Easy mode: the automation as one sentence, each part a button, and the panel for the part
// that's picked below it. It edits the same automation as Advanced; what it can't show
// (easyExtras) stays as it is and is listed with a way to open Advanced.
defineProps<{ otherAvatar: boolean }>()
const emit = defineEmits<{ advanced: [] }>()
const automation = defineModel<Automation>({ required: true })

const { describeContext } = useAutomations()
const { offAddonForAutomationKind } = useAddons()

const words = computed(() => sentence(automation.value, describeContext.value))
const extras = computed(() => easyExtras(automation.value))
const condition = computed(() => triggerCondition(automation.value.when))
const whenMissing = computed(() => automation.value.when.kind === 'avatar-parameter' && !automation.value.when.parameter)

// Opens on the first part still to fill in.
const firstMissing = automation.value.then.findIndex(stepMissing)
const selected = ref(whenMissing.value || firstMissing < 0 ? 'when' : `step-${firstMissing}`)
const stepIndex = computed(() => (selected.value.startsWith('step-') ? Number(selected.value.slice(5)) : -1))

// A new step starts a second after the last, when it can wait.
const addStep = (): void => {
  if (automation.value.then.length >= MAX_STEPS) return
  const step = newStep(stepKindsFor(condition.value, (kind) => !offAddonForAutomationKind(kind))[0]!, condition.value)
  if (stepCanWait(step)) step.delayMs = 1000
  automation.value.then.push(step)
  selected.value = `step-${automation.value.then.length - 1}`
}

const removeStep = (index: number): void => {
  automation.value.then.splice(index, 1)
  selected.value = index ? `step-${index - 1}` : 'when'
}
</script>

<template>
  <EasySentence
    v-model:selected="selected"
    :sentence="words"
    :when-icon="triggerIcon(automation.when)"
    :when-missing="whenMissing"
    :step-icons="automation.then.map((step) => STEP_KINDS[step.kind].icon)"
    :can-add-step="automation.then.length < MAX_STEPS"
    :extras="extras.length"
    @add-step="addStep"
    @advanced="emit('advanced')"
  />

  <UAlert
    v-if="extras.length"
    color="warning"
    variant="subtle"
    icon="i-lucide-sliders-horizontal"
    title="This automation also uses settings Easy mode can't show."
    :description="`${extras.join(' · ')}. They stay as they are; change them in Advanced.`"
  />

  <EasyWhenPanel
    v-if="selected === 'when'"
    v-model="automation"
    :other-avatar="otherAvatar"
  />
  <EasyStepPanel
    v-else-if="automation.then[stepIndex]"
    :key="stepIndex"
    v-model="automation.then[stepIndex]!"
    :index="stepIndex"
    :condition="condition"
    :trigger="automation.when.kind === 'avatar-parameter' ? automation.when.parameter : ''"
    :can-remove="automation.then.length > 1"
    @remove="removeStep(stepIndex)"
  />
  <EasyOnlyPanel
    v-else-if="selected === 'only'"
    v-model="automation.onlyIf"
    v-model:any="automation.onlyIfAny"
    :other-avatar="otherAvatar"
  />
  <EasyOftenPanel
    v-else-if="selected === 'often'"
    v-model="automation"
  />
</template>
