<script setup lang="ts">
import OptionGrid from '@renderer/components/ui/OptionGrid.vue'
import SegmentedControl from '@renderer/components/ui/SegmentedControl.vue'
import type { VoiceFxEvent, VoiceFxStarter, VoiceFxTrigger } from '@renderer/lib/tauri-bridge'
import { VOICE_FX_EVENTS, VOICE_FX_STARTERS } from '@renderer/utils/automations'
import { computed } from 'vue'

// The fields of the Voice effects trigger: an effect turns on, off or changes, whoever started
// it or only one source.
const when = defineModel<VoiceFxTrigger>({ required: true })

const EVENTS = (Object.keys(VOICE_FX_EVENTS) as VoiceFxEvent[]).map((event) => ({ value: event, label: VOICE_FX_EVENTS[event].label }))

const ANYONE = 'anyone'
const STARTERS = [{ value: ANYONE, label: 'Anyone' }, ...(Object.keys(VOICE_FX_STARTERS) as VoiceFxStarter[]).map((source) => ({ value: source as string, label: VOICE_FX_STARTERS[source] }))]
const starter = computed({
  get: () => when.value.source ?? ANYONE,
  set: (value: string) => (when.value.source = value === ANYONE ? null : (value as VoiceFxStarter))
})
</script>

<template>
  <div class="grid gap-4">
    <UFormField
      label="When a voice effect…"
      :description="VOICE_FX_EVENTS[when.event].description"
    >
      <SegmentedControl
        v-model="when.event"
        class="w-fit max-w-full flex-wrap"
        accent
        :items="EVENTS"
        aria-label="Voice effect event"
      />
    </UFormField>
    <UFormField
      label="Started by"
      description="A voice effects step can't follow this one: it would set itself off again."
    >
      <OptionGrid
        v-model="starter"
        min-width="9rem"
        :items="STARTERS"
        aria-label="Started by"
      />
    </UFormField>
  </div>
</template>
