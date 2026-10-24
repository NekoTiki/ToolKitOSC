<script setup lang="ts">
import type { ServerSettings } from '~~/server/utils/serverSettings'

useHead({ title: 'Settings · Dashboard' })

const { adminFetch, redirectIfUnauthorized } = useAdminApi()
const toast = useToast()

const { data, error } = await useFetch<{ settings: ServerSettings; providers: { id: string; label: string }[] }>('/api/admin/settings')

if (error.value) await redirectIfUnauthorized(error.value)

// Edited here, saved together.
const form = ref<ServerSettings | null>(data.value ? { ...data.value.settings } : null)
const busy = ref(false)

const dirty = computed(() => !!form.value && !!data.value && JSON.stringify(form.value) !== JSON.stringify(data.value.settings))

const providerItems = computed(() => [{ label: 'Spread across all', value: '' }, ...(data.value?.providers ?? []).map((provider) => ({ label: provider.label, value: provider.id }))])
const profileItems = [
  { label: 'Light', value: 'light' as const },
  { label: 'Balanced', value: 'balanced' as const },
  { label: 'Heavy', value: 'heavy' as const }
]

const save = async (): Promise<void> => {
  if (!form.value || !data.value) return

  busy.value = true

  try {
    const { settings } = await adminFetch<{ settings: ServerSettings }>('/api/admin/settings', { method: 'PUT', body: form.value })

    data.value.settings = settings
    form.value = { ...settings }
    toast.add({ title: 'Settings saved', icon: 'i-lucide-check', color: 'success' })
  } finally {
    busy.value = false
  }
}
</script>

<template>
  <div class="grid gap-4">
    <div class="flex flex-wrap items-center gap-3">
      <div class="grid flex-1 gap-0.5">
        <h1 class="text-[28px] leading-tight font-semibold text-highlighted">
          Settings
        </h1>
        <p class="text-[13px] text-muted">
          They apply right away, without a restart
        </p>
      </div>
      <UButton
        icon="i-lucide-check"
        :disabled="!dirty"
        :loading="busy"
        @click="save"
      >
        Save
      </UButton>
    </div>

    <template v-if="form">
      <UCard :ui="{ body: 'grid gap-4' }">
        <h2 class="text-[17px] font-semibold text-highlighted">
          AI suggestions
        </h2>
        <USwitch
          v-model="form.aiEnabled"
          label="AI suggestions are on"
          description="Off hides them for everyone, admins included. Who has access stays as it is."
        />
        <UFormField
          label="Credits per day"
          description="Each account's pool, reset at midnight UTC. A light suggestion costs 1, balanced 2, heavy 4."
        >
          <UInputNumber
            v-model="form.dailyCredits"
            :min="0"
            :max="10000"
            class="w-36"
          />
        </UFormField>
        <UFormField
          label="Provider"
          description="For accounts that can't pick a model."
        >
          <AdminSegmented
            v-model="form.defaultProvider"
            :items="providerItems"
            label="Provider"
            size="sm"
          />
        </UFormField>
        <UFormField
          label="Model size"
          description="For accounts that can't pick a model."
        >
          <AdminSegmented
            v-model="form.defaultProfile"
            :items="profileItems"
            label="Model size"
            size="sm"
          />
        </UFormField>
      </UCard>

      <UCard :ui="{ body: 'grid gap-4 sm:grid-cols-2' }">
        <h2 class="text-[17px] font-semibold text-highlighted sm:col-span-2">
          Share pages
        </h2>
        <UFormField
          label="Viewers per share page"
          description="New viewers see that the page is full."
        >
          <UInputNumber
            v-model="form.maxViewersPerRoom"
            :min="1"
            :max="100000"
            class="w-36"
          />
        </UFormField>
        <UFormField
          label="Open pages per network"
          description="Across every share page, from one IP address."
        >
          <UInputNumber
            v-model="form.maxSocketsPerIp"
            :min="1"
            :max="1000"
            class="w-36"
          />
        </UFormField>
      </UCard>

      <UCard :ui="{ body: 'grid gap-4' }">
        <h2 class="text-[17px] font-semibold text-highlighted">
          Data
        </h2>
        <UFormField
          label="Keep stats for (days)"
          description="AI generations, credits and control use older than this are deleted once a day. The admin log is kept."
        >
          <UInputNumber
            v-model="form.retentionDays"
            :min="7"
            :max="3650"
            class="w-36"
          />
        </UFormField>
      </UCard>
    </template>
  </div>
</template>
