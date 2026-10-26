<script setup lang="ts">
// Suspending an account and signing it out of every app.
const props = defineProps<{ discordId: string; suspendedUntil: string | null; suspendReason: string | null }>()
const emit = defineEmits<{ changed: [] }>()

const { adminFetch } = useAdminApi()
const toast = useToast()

const FOREVER_YEAR = 9999

const suspended = computed(() => !!props.suspendedUntil && new Date(props.suspendedUntil) > new Date())
const forever = computed(() => !!props.suspendedUntil && new Date(props.suspendedUntil).getUTCFullYear() >= FOREVER_YEAR)

const durationItems = [
  { label: '1 day', value: 1 },
  { label: '7 days', value: 7 },
  { label: '30 days', value: 30 },
  { label: 'Until lifted', value: 0 }
]

const confirming = ref<'suspend' | 'sign-out' | null>(null)
const days = ref(7)
const reason = ref('')
const busy = ref(false)

const run = async (action: () => Promise<unknown>, title: string): Promise<void> => {
  busy.value = true

  try {
    await action()
    toast.add({ title, icon: 'i-lucide-check', color: 'success' })
    emit('changed')
  } finally {
    busy.value = false
    confirming.value = null
  }
}

const suspend = () =>
  run(
    () => adminFetch(`/api/admin/users/${props.discordId}/suspend`, { method: 'POST', body: { days: days.value || null, reason: reason.value || undefined } }),
    'Account suspended'
  )
const lift = () => run(() => adminFetch(`/api/admin/users/${props.discordId}/suspend`, { method: 'DELETE' }), 'Suspension lifted')
const signOut = () => run(() => adminFetch(`/api/admin/users/${props.discordId}/sign-out`, { method: 'POST' }), 'Signed out everywhere')
</script>

<template>
  <UCard :ui="{ body: 'grid gap-3' }">
    <div class="flex flex-wrap items-center gap-2.5">
      <h2 class="flex-1 text-[17px] font-semibold text-highlighted">
        Account
      </h2>
      <UBadge
        :color="suspended ? 'error' : 'success'"
        variant="subtle"
      >
        {{ suspended ? 'Suspended' : 'Active' }}
      </UBadge>
    </div>

    <p
      v-if="suspended"
      class="text-sm"
    >
      Suspended
      <template v-if="forever">
        until you lift it
      </template>
      <template v-else>
        until <NuxtTime
          :datetime="suspendedUntil!"
          day="numeric"
          month="short"
          hour="2-digit"
          minute="2-digit"
        />
      </template>
      <span v-if="suspendReason"> · {{ suspendReason }}</span>
    </p>
    <p
      v-else
      class="text-[13px] text-muted"
    >
      Suspending signs their app out and keeps it out. On share pages they count as a guest.
    </p>

    <div
      v-if="confirming === 'suspend'"
      class="grid gap-3 rounded-field border border-error/40 bg-error/10 p-3"
    >
      <AdminSegmented
        v-model="days"
        :items="durationItems"
        label="How long"
        size="sm"
      />
      <UInput
        v-model="reason"
        placeholder="Reason, shown to them"
        :maxlength="300"
      />
      <div class="flex flex-wrap gap-2">
        <UButton
          color="neutral"
          variant="subtle"
          @click="confirming = null"
        >
          Cancel
        </UButton>
        <UButton
          color="error"
          :loading="busy"
          @click="suspend"
        >
          Suspend
        </UButton>
      </div>
    </div>

    <div
      v-else-if="confirming === 'sign-out'"
      class="flex flex-wrap items-center gap-2 rounded-field border border-error/40 bg-error/10 p-3 text-sm"
    >
      <span class="flex-1">Sign them out of every app? They can sign in again.</span>
      <UButton
        color="neutral"
        variant="subtle"
        @click="confirming = null"
      >
        Cancel
      </UButton>
      <UButton
        color="error"
        :loading="busy"
        @click="signOut"
      >
        Sign out everywhere
      </UButton>
    </div>

    <div
      v-else
      class="flex flex-wrap gap-2"
    >
      <UButton
        v-if="suspended"
        icon="i-lucide-undo-2"
        color="neutral"
        variant="subtle"
        :loading="busy"
        @click="lift"
      >
        Lift suspension
      </UButton>
      <UButton
        v-else
        icon="i-lucide-ban"
        color="error"
        variant="soft"
        @click="confirming = 'suspend'"
      >
        Suspend
      </UButton>
      <UButton
        icon="i-lucide-log-out"
        color="neutral"
        variant="subtle"
        @click="confirming = 'sign-out'"
      >
        Sign out everywhere
      </UButton>
    </div>
  </UCard>
</template>
