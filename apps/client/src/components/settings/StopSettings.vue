<script setup lang="ts">
import type { StatusInfo } from '@renderer/components/settings/StatusLine.vue'
import StatusLine from '@renderer/components/settings/StatusLine.vue'
import FormSection from '@renderer/components/ui/FormSection.vue'
import { useAvatarDetails } from '@renderer/composables/useAvatarDetails'
import { useOscMessages } from '@renderer/composables/useOscMessages'
import {
  DEFAULT_STOP_HOTKEY,
  formatHotkey,
  PANIC_ADDRESS,
  PANIC_PARAMETER,
  useStopEverything
} from '@renderer/composables/useStopEverything'
import { computed, ref } from 'vue'

const { paused, hotkey, hotkeyError, toggle, setHotkey } = useStopEverything()
const { avatarDetails } = useAvatarDetails()
const { args } = useOscMessages()
const toast = useToast()

// Recording: the next key pressed with at least one modifier becomes the hotkey. F keys and Pause
// are allowed alone, since nothing types them.
const recording = ref(false)

const MODIFIERS = new Set(['Control', 'Shift', 'Alt', 'Meta'])
const ALONE = /^(F\d{1,2}|Pause)$/

const onKeydown = (event: KeyboardEvent): void => {
  if (!recording.value) return

  event.preventDefault()

  if (event.key === 'Escape') {
    recording.value = false
    return
  }

  if (MODIFIERS.has(event.key)) return

  const modifiers = [
    event.ctrlKey && 'CommandOrControl',
    event.altKey && 'Alt',
    event.shiftKey && 'Shift',
    event.metaKey && 'Super'
  ].filter((part): part is string => !!part)

  if (!modifiers.length && !ALONE.test(event.code)) return

  recording.value = false
  void setHotkey([...modifiers, event.code].join('+'))
}

const hotkeyStatus = computed<StatusInfo>(() => {
  if (hotkeyError.value) return { icon: 'i-lucide-circle-x', class: 'text-error', label: hotkeyError.value }
  if (!hotkey.value) return { icon: 'i-lucide-circle-dashed', class: 'text-muted', label: 'Off' }

  return { icon: 'i-lucide-circle-check', class: 'text-success', label: 'Works from any app, VRChat included' }
})

// Read from the parameters VRChat reported for the avatar you're wearing.
const parameterStatus = computed<StatusInfo>(() => {
  if (!avatarDetails.value) return { icon: 'i-lucide-circle-dashed', class: 'text-muted', label: 'Waiting for VRChat' }
  if (args.value[PANIC_ADDRESS] !== undefined) return { icon: 'i-lucide-circle-check', class: 'text-success', label: 'Found on this avatar' }

  return { icon: 'i-lucide-circle-dashed', class: 'text-muted', label: 'Not on this avatar' }
})

const copyParameter = async (): Promise<void> => {
  try {
    await navigator.clipboard.writeText(PANIC_PARAMETER)
    toast.add({ title: 'Parameter name copied', icon: 'i-lucide-check', color: 'success' })
  } catch {
    toast.add({ title: 'Could not copy the parameter name', color: 'error' })
  }
}
</script>

<template>
  <FormSection
    title="Stop everything"
    description="Stops every toy, pattern and shocker at once, and pauses your share page. Viewers see a paused card and can't use your controls until you resume. Your own controls in this app keep working."
  >
    <UButton
      size="xl"
      block
      :icon="paused ? 'i-lucide-play' : 'i-lucide-octagon-x'"
      :color="paused ? 'warning' : 'error'"
      :variant="paused ? 'subtle' : 'solid'"
      @click="toggle"
    >
      {{ paused ? 'Paused · Resume' : 'Stop everything' }}
    </UButton>
  </FormSection>

  <FormSection
    title="Hotkey"
    description="Stops everything, or resumes when paused, even while another app is in front."
  >
    <div class="flex flex-wrap items-center gap-2">
      <UButton
        size="lg"
        color="neutral"
        variant="subtle"
        icon="i-lucide-keyboard"
        class="min-w-48 font-mono"
        @click="recording = true"
        @keydown="onKeydown"
        @blur="recording = false"
      >
        {{ recording ? 'Press the keys…' : hotkey ? formatHotkey(hotkey) : 'Off' }}
      </UButton>
      <UButton
        v-if="hotkey !== DEFAULT_STOP_HOTKEY"
        size="lg"
        color="neutral"
        variant="ghost"
        icon="i-lucide-rotate-ccw"
        @click="setHotkey(DEFAULT_STOP_HOTKEY)"
      >
        Reset to {{ formatHotkey(DEFAULT_STOP_HOTKEY) }}
      </UButton>
      <UButton
        v-if="hotkey"
        size="lg"
        color="neutral"
        variant="ghost"
        icon="i-lucide-x"
        @click="setHotkey('')"
      >
        Turn off
      </UButton>
    </div>
    <StatusLine :status="hotkeyStatus" />
  </FormSection>

  <FormSection
    title="Avatar parameter"
    description="Add a Bool parameter with this name to your avatar and a toggle for it in your expression menu. Turning it on stops everything. Turning it off doesn't resume, so a slip in the menu can't give control back: resume here or with the hotkey."
  >
    <div class="flex min-h-13.5 items-center gap-2 rounded-field well py-1.5 pr-1.5 pl-3.5 font-mono text-[13px]">
      <span class="min-w-0 flex-1 truncate">{{ PANIC_PARAMETER }}</span>
      <UButton
        icon="i-lucide-copy"
        size="md"
        color="neutral"
        variant="subtle"
        @click="copyParameter"
      >
        Copy
      </UButton>
    </div>
    <StatusLine :status="parameterStatus" />
  </FormSection>
</template>
