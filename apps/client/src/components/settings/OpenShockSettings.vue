<script setup lang="ts">
import type { StatusInfo } from '@renderer/components/settings/StatusLine.vue'
import StatusLine from '@renderer/components/settings/StatusLine.vue'
import FormSection from '@renderer/components/ui/FormSection.vue'
import { useOpenShock } from '@renderer/composables/useOpenShock'
import { computed, ref } from 'vue'

const { hasToken, status, setToken, enabled, setEnabled, shockerNames, state } = useOpenShock()
const toast = useToast()

// The saved key stays in the app (the system's password store), so this field only ever holds a
// new one being pasted in.
const draft = ref('')
const visible = ref(false)
const replacing = ref(false)
const saving = ref(false)

const save = async (value: string): Promise<void> => {
  saving.value = true
  try {
    await setToken(value)
    draft.value = ''
    replacing.value = false
  } catch (error) {
    toast.add({ title: 'Could not save the API key', description: String(error), color: 'error' })
  } finally {
    saving.value = false
  }
}

// Committing re-validates the key against OpenShock, so it only happens on change/Enter/blur.
const commit = (): void => {
  if (draft.value.trim() && !saving.value) void save(draft.value)
}

const keyDescription = computed(() => {
  if (!hasToken.value || replacing.value) return 'Create one in your OpenShock account settings.'

  return state.value?.secretStore === 'file' ? 'Saved in a file, no password store found.' : "Saved in your system's password store."
})

const statusInfo = computed<StatusInfo>(() => {
  switch (status.value) {
    case 'checking':
      return { icon: 'i-lucide-circle-dashed', class: 'text-muted animate-pulse', label: 'Checking key…' }
    case 'valid':
      return { icon: 'i-lucide-circle-check', class: 'text-success', label: 'Connected' }
    case 'invalid':
      return { icon: 'i-lucide-circle-x', class: 'text-error', label: 'Invalid key' }
    case 'disabled':
      return { icon: 'i-lucide-circle-dashed', class: 'text-muted', label: 'Disabled' }
    default:
      return { icon: 'i-lucide-circle-dashed', class: 'text-muted', label: 'Not configured' }
  }
})

const shockers = computed(() => Array.from(shockerNames.value.entries()).map(([id, name]) => ({ id, name })))
</script>

<template>
  <FormSection>
    <USwitch
      :model-value="enabled"
      label="Enable OpenShock"
      description="Lets you create and use Shocker controls."
      @update:model-value="setEnabled(!!$event)"
    />

    <template v-if="enabled">
      <UFormField
        label="API key"
        :description="keyDescription"
      >
        <div
          v-if="hasToken && !replacing"
          class="flex gap-2"
        >
          <UButton
            label="Replace"
            icon="i-lucide-key-round"
            color="neutral"
            variant="outline"
            @click="replacing = true"
          />
          <UButton
            label="Remove"
            icon="i-lucide-trash-2"
            color="error"
            variant="soft"
            :loading="saving"
            @click="save('')"
          />
        </div>
        <UInput
          v-else
          v-model="draft"
          :type="visible ? 'text' : 'password'"
          placeholder="Paste your OpenShock API key"
          class="w-full"
          autocomplete="off"
          :disabled="saving"
          :ui="{ base: 'font-mono' }"
          @change="commit"
          @keyup.enter="commit"
          @blur="commit"
        >
          <template #trailing>
            <UButton
              :icon="visible ? 'i-lucide-eye-off' : 'i-lucide-eye'"
              color="neutral"
              variant="link"
              size="sm"
              :aria-label="visible ? 'Hide API key' : 'Show API key'"
              @click="visible = !visible"
            />
          </template>
        </UInput>
      </UFormField>
      <StatusLine :status="statusInfo" />
    </template>
  </FormSection>

  <FormSection
    v-if="enabled && status === 'valid'"
    title="Shockers"
    :description="`${shockers.length} on this account`"
  >
    <div class="grid divide-y divide-(--ui-border)">
      <div
        v-for="shocker in shockers"
        :key="shocker.id"
        class="flex min-h-13 items-center gap-3"
      >
        <span class="grid size-10 place-items-center rounded-md bg-(--aurora-well) text-muted">
          <UIcon
            name="material-symbols:electric-bolt"
            class="size-5"
          />
        </span>
        <b class="font-medium text-highlighted">{{ shocker.name }}</b>
      </div>
      <p
        v-if="!shockers.length"
        class="py-1 text-sm text-muted"
      >
        No shockers found on this account.
      </p>
    </div>
  </FormSection>
</template>
