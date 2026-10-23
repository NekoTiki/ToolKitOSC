<script setup lang="ts">
import AutomationStepCard from '@renderer/components/automations/AutomationStepCard.vue'
import SegmentedControl from '@renderer/components/ui/SegmentedControl.vue'
import { useAutomations } from '@renderer/composables/useAutomations'
import { useChatbox } from '@renderer/composables/useChatbox'
import type {
  AutomationAction,
  AutomationEnabledAction,
  AutomationStep,
  OneOfAction,
  OscMessageAction,
  VrToastAction
} from '@renderer/lib/tauri-bridge'
import { CHATBOX_MAX, MAX_ONE_OF, newStep, PLACEHOLDER_HINT } from '@renderer/utils/automations'
import { computed } from 'vue'

// The fields of the "turn an automation on or off", VR message, "one of these" and OSC message
// steps, for both editors.
const props = defineProps<{
  // The automation being edited: it can't turn itself on or off.
  automationId: string
  trigger: string
  otherAvatar: boolean
}>()
const step = defineModel<AutomationEnabledAction | VrToastAction | OneOfAction | OscMessageAction>({ required: true })

const { automations } = useAutomations()
const { enabled: chatboxEnabled } = useChatbox()

// ---- Automation on/off

const toggled = computed(() => (step.value.kind === 'automation-enabled' ? step.value : null))
const automationItems = computed(() =>
  automations.value.filter((a) => a.id !== props.automationId).map((a) => ({ label: a.name, value: a.id }))
)

// ---- VR message

const toast = computed(() => (step.value.kind === 'vr-toast' ? step.value : null))

// ---- One of these

const oneOf = computed(() => (step.value.kind === 'one-of' ? step.value : null))
// An option is a step without a delay: they all run right away.
const withoutDelay = (value: AutomationStep): AutomationAction => {
  const action: Partial<AutomationStep> = { ...value }
  delete action.delayMs
  return action as AutomationAction
}
const setOption = (index: number, value: AutomationStep): void => {
  if (oneOf.value) oneOf.value.options[index] = withoutDelay(value)
}
const addOption = (): void => {
  oneOf.value?.options.push(withoutDelay(newStep('chatbox', 'turns-on')))
}

// ---- OSC message

const osc = computed(() => (step.value.kind === 'osc-message' ? step.value : null))
const oscType = computed({
  get: (): 'bool' | 'number' | 'text' =>
    typeof osc.value?.value === 'boolean' ? 'bool' : typeof osc.value?.value === 'number' ? 'number' : 'text',
  set: (type: 'bool' | 'number' | 'text') => {
    if (osc.value) osc.value.value = type === 'bool' ? true : type === 'number' ? 1 : ''
  }
})
</script>

<template>
  <div class="grid gap-4">
    <template v-if="toggled">
      <UFormField label="Automation">
        <USelectMenu
          v-model="toggled.automationId"
          :items="automationItems"
          value-key="value"
          placeholder="Pick an automation"
          class="w-full"
        />
      </UFormField>
      <SegmentedControl
        :model-value="toggled.enabled ? 'on' : 'off'"
        :items="[
          { value: 'on', label: 'Turn it on' },
          { value: 'off', label: 'Turn it off' }
        ]"
        aria-label="On or off"
        class="w-fit"
        @update:model-value="toggled.enabled = $event === 'on'"
      />
    </template>

    <UFormField
      v-else-if="toast"
      label="Message"
      :hint="`${toast.text.length} / ${CHATBOX_MAX}`"
      :description="PLACEHOLDER_HINT"
    >
      <UInput
        v-model="toast.text"
        :maxlength="CHATBOX_MAX"
        placeholder="{viewer} booped you"
        class="w-full"
        autocomplete="off"
      />
    </UFormField>

    <div
      v-else-if="oneOf"
      class="grid gap-2.5"
    >
      <AutomationStepCard
        v-for="(option, index) in oneOf.options"
        :key="index"
        :model-value="option as AutomationStep"
        :index="index"
        condition="turns-on"
        :trigger="trigger"
        :other-avatar="otherAvatar"
        :automation-id="automationId"
        :can-remove="oneOf.options.length > 2"
        nested
        @update:model-value="setOption(index, $event)"
        @remove="oneOf.options.splice(index, 1)"
      />
      <div v-if="oneOf.options.length < MAX_ONE_OF">
        <UButton
          icon="i-lucide-plus"
          color="neutral"
          variant="subtle"
          @click="addOption"
        >
          Add an option
        </UButton>
      </div>
      <p
        v-if="oneOf.options.some((o) => o.kind === 'chatbox') && !chatboxEnabled"
        class="text-sm text-error"
      >
        The chatbox is off, so chatbox options won't post anything.
      </p>
    </div>

    <template v-else-if="osc">
      <UAlert
        color="warning"
        variant="subtle"
        icon="i-lucide-triangle-alert"
        title="Advanced"
        description="Sent to VRChat as it is. Nothing checks what the address does."
      />
      <UFormField
        label="Address"
        description="Starts with /, like /input/Jump or /avatar/parameters/Something."
      >
        <UInput
          v-model="osc.address"
          class="w-full font-mono"
          autocomplete="off"
        />
      </UFormField>
      <UFormField label="Value">
        <div class="flex flex-wrap items-center gap-3">
          <SegmentedControl
            v-model="oscType"
            :items="[
              { value: 'bool', label: 'On/off' },
              { value: 'number', label: 'Number' },
              { value: 'text', label: 'Text' }
            ]"
            aria-label="Value type"
            class="w-fit"
          />
          <SegmentedControl
            v-if="typeof osc.value === 'boolean'"
            :model-value="osc.value ? 'on' : 'off'"
            :items="[
              { value: 'on', label: 'On' },
              { value: 'off', label: 'Off' }
            ]"
            aria-label="Value"
            class="w-fit"
            @update:model-value="osc && (osc.value = $event === 'on')"
          />
          <UInput
            v-else-if="typeof osc.value === 'number'"
            :model-value="osc.value"
            type="number"
            step="any"
            class="w-32"
            @update:model-value="osc && (osc.value = Number($event) || 0)"
          />
          <UInput
            v-else
            v-model="osc.value"
            :maxlength="CHATBOX_MAX"
            class="min-w-48 flex-1"
          />
        </div>
      </UFormField>
    </template>
  </div>
</template>
