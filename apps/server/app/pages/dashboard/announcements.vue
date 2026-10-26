<script setup lang="ts">
import type { ServerNotice } from '#shared/types/protocol'

useHead({ title: 'Announcements · Dashboard' })

const { adminFetch, redirectIfUnauthorized } = useAdminApi()
const toast = useToast()

const { data, error, refresh } = await useFetch<{ notice: ServerNotice | null }>('/api/admin/announcement')

if (error.value) await redirectIfUnauthorized(error.value)

const text = ref('')
const level = ref<'info' | 'warning'>('info')
const minutes = ref<number | undefined>(60)
const onSharePages = ref(false)
const busy = ref(false)

const levelItems = [
  { label: 'Info', value: 'info' as const },
  { label: 'Warning', value: 'warning' as const }
]

const send = async (): Promise<void> => {
  busy.value = true

  try {
    await adminFetch('/api/admin/announcement', {
      method: 'POST',
      body: { text: text.value, level: level.value, minutes: minutes.value || undefined, onSharePages: onSharePages.value }
    })
    toast.add({ title: 'Announcement sent', icon: 'i-lucide-megaphone', color: 'success' })
    text.value = ''
    await refresh()
  } finally {
    busy.value = false
  }
}

const clear = async (): Promise<void> => {
  busy.value = true

  try {
    await adminFetch('/api/admin/announcement', { method: 'DELETE' })
    toast.add({ title: 'Announcement cleared', icon: 'i-lucide-check' })
    await refresh()
  } finally {
    busy.value = false
  }
}
</script>

<template>
  <div class="grid gap-4">
    <div class="grid gap-0.5">
      <h1 class="text-[28px] leading-tight font-semibold text-highlighted">
        Announcements
      </h1>
      <p class="text-[13px] text-muted">
        A message every host sees in the app, for example before a restart
      </p>
    </div>

    <UCard
      v-if="data?.notice"
      :ui="{ body: 'grid gap-3' }"
    >
      <div class="flex flex-wrap items-center gap-2.5">
        <h2 class="flex-1 text-[17px] font-semibold text-highlighted">
          Showing now
        </h2>
        <UBadge
          :color="data.notice.level === 'warning' ? 'warning' : 'info'"
          variant="subtle"
        >
          {{ data.notice.level === 'warning' ? 'Warning' : 'Info' }}
        </UBadge>
        <UBadge
          v-if="data.notice.onSharePages"
          color="neutral"
          variant="subtle"
        >
          Also on share pages
        </UBadge>
      </div>
      <p>{{ data.notice.text }}</p>
      <p class="text-[13px] text-muted">
        <template v-if="data.notice.expiresAt">
          Ends <NuxtTime
            :datetime="data.notice.expiresAt"
            relative
          />
        </template>
        <template v-else>
          Shown until you clear it
        </template>
      </p>
      <UButton
        icon="i-lucide-x"
        color="neutral"
        variant="subtle"
        class="w-fit"
        :loading="busy"
        @click="clear"
      >
        Clear
      </UButton>
    </UCard>

    <UCard :ui="{ body: 'grid gap-3.5' }">
      <h2 class="text-[17px] font-semibold text-highlighted">
        {{ data?.notice ? 'Replace it' : 'New announcement' }}
      </h2>
      <UFormField label="Message">
        <UTextarea
          v-model="text"
          placeholder="The server restarts in 5 minutes. You'll reconnect by yourself."
          class="w-full"
          :maxlength="500"
          :rows="3"
        />
      </UFormField>
      <div class="flex flex-wrap items-end gap-4">
        <UFormField label="Type">
          <AdminSegmented
            v-model="level"
            :items="levelItems"
            label="Type"
          />
        </UFormField>
        <UFormField
          label="Show for (minutes)"
          description="Empty: until you clear it."
        >
          <UInputNumber
            v-model="minutes"
            :min="1"
            :max="10080"
            class="w-36"
          />
        </UFormField>
      </div>
      <USwitch
        v-model="onSharePages"
        label="Also show it on share pages"
        description="Viewers see it as a banner above the controls."
      />
      <UButton
        icon="i-lucide-megaphone"
        class="w-fit"
        :disabled="!text.trim()"
        :loading="busy"
        @click="send"
      >
        Send
      </UButton>
    </UCard>
  </div>
</template>
