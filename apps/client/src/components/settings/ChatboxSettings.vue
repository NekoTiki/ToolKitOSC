<script setup lang="ts">
import FormSection from '@renderer/components/ui/FormSection.vue'
import { CHATBOX_KINDS, useChatbox } from '@renderer/composables/useChatbox'
import { useShareSettings } from '@renderer/composables/useShareSettings'
import { computed } from 'vue'

const { enabled, kinds, setEnabled, setKind, sendTest } = useChatbox()
const { showViewers } = useShareSettings()

// Without names, everyone's actions merge into one sentence.
const preview = computed(() =>
  showViewers.value
    ? 'Alex turned on Ears, set Tail to 40% · Sam set Face to Blush'
    : 'Someone turned on Ears, set Tail to 40%, set Face to Blush'
)
</script>

<template>
  <FormSection>
    <USwitch
      :model-value="enabled"
      label="Post viewer actions to the chatbox"
      description="Everyone in your instance sees who is controlling you, above your head. Actions that arrive close together are merged into one message every few seconds. Pausing and resuming are posted too."
      @update:model-value="setEnabled(!!$event)"
    />
  </FormSection>

  <template v-if="enabled">
    <FormSection
      title="What to post"
      description="Your own taps in this app are never posted."
    >
      <div class="grid gap-2.5 sm:grid-cols-2">
        <UCheckbox
          v-for="kind in CHATBOX_KINDS"
          :key="kind.value"
          :model-value="kinds.includes(kind.value)"
          :label="kind.label"
          size="lg"
          @update:model-value="setKind(kind.value, !!$event)"
        />
      </div>
    </FormSection>

    <FormSection
      title="Preview"
      :description="
        showViewers
          ? 'Viewer names are shown. Turn off Show viewers to each other in Settings > Sharing & access to hide them.'
          : 'Show viewers to each other is off in Settings > Sharing & access, so names are hidden here too.'
      "
    >
      <div class="rounded-field well px-3.5 py-3 text-[14.5px]">
        {{ preview }}
      </div>
      <div>
        <UButton
          icon="i-lucide-send"
          color="neutral"
          variant="subtle"
          @click="sendTest"
        >
          Send a test message
        </UButton>
      </div>
    </FormSection>
  </template>
</template>
