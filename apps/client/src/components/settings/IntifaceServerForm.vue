<script setup lang="ts">
import { ref } from 'vue'

// Adding or editing an Intiface server: your name for it, its address and port. Shown in place on
// Settings > Intiface, not in a dialog.
const props = defineProps<{
  initial?: { label: string; host: string; port: number }
  defaultPort: number
  submitLabel: string
  // Resolves when saved; rejects with the message to show (bad or duplicate address).
  save: (label: string, host: string, port: number) => Promise<unknown>
}>()

const emit = defineEmits<{ done: [] }>()

const label = ref(props.initial?.label ?? '')
const host = ref(props.initial?.host ?? '')
const port = ref<number>(props.initial?.port ?? props.defaultPort)
const error = ref('')
const saving = ref(false)

const submit = async (): Promise<void> => {
  const value = Math.round(Number(port.value))

  if (!host.value.trim()) {
    error.value = 'Enter the address of the device running Intiface.'
    return
  }
  if (!(value >= 1 && value <= 65535)) {
    error.value = 'The port is a number from 1 to 65535.'
    return
  }

  saving.value = true
  error.value = ''

  try {
    await props.save(label.value, host.value, value)
    emit('done')
  } catch (reason) {
    error.value = String(reason)
  } finally {
    saving.value = false
  }
}
</script>

<template>
  <form
    class="grid gap-3 rounded-lg bg-(--aurora-well) p-3.5"
    @submit.prevent="submit"
  >
    <UFormField
      label="Name"
      description="Shown next to its toys. Leave empty to show the address."
    >
      <UInput
        v-model="label"
        placeholder="Bedroom phone"
        class="w-full"
        autocomplete="off"
        :maxlength="60"
      />
    </UFormField>
    <div class="grid gap-3 sm:grid-cols-[minmax(0,1fr)_8rem]">
      <UFormField
        label="Address"
        required
      >
        <UInput
          v-model="host"
          placeholder="192.168.1.20"
          class="w-full"
          autocomplete="off"
          :ui="{ base: 'font-mono' }"
        />
      </UFormField>
      <UFormField
        label="Port"
        required
      >
        <UInput
          v-model.number="port"
          type="number"
          :min="1"
          :max="65535"
          class="w-full"
          :ui="{ base: 'font-mono' }"
        />
      </UFormField>
    </div>
    <p
      v-if="error"
      class="text-sm text-error"
      role="alert"
    >
      {{ error }}
    </p>
    <div class="flex gap-2">
      <UButton
        type="submit"
        :loading="saving"
      >
        {{ submitLabel }}
      </UButton>
      <UButton
        color="neutral"
        variant="subtle"
        @click="emit('done')"
      >
        Cancel
      </UButton>
    </div>
  </form>
</template>
