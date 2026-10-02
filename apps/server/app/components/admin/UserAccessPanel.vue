<script setup lang="ts">
import type { AccessEntry } from '~/composables/useAdminUserAccess'

// Everything an admin can change about one user's AI access, together on their page: grant or
// revoke (now with a confirmation step), model choice, today's credits and the admin-only note.
// Previously spread across the users list as inline controls, with no note field and no
// confirmation before revoking.
const DAILY_CREDITS = 20

const props = defineProps<{ discordId: string }>()

const { data, refresh } = await useFetch<{ users: AccessEntry[] }>('/api/admin/users')

const entry = computed(() => data.value?.users.find((user) => user.discordId === props.discordId) ?? null)

const { grant, revoke, setCanSelectModel, saveNote, addCredits } = useAdminUserAccess(refresh)

const note = ref('')
const credits = ref(5)
const confirmingRevoke = ref(false)
const busy = ref(false)

watch(entry, (value) => (note.value = value?.note ?? ''), { immediate: true })

// Every action disables the panel until the reload finishes, so double clicks don't stack.
const run = async (action: () => Promise<void>): Promise<void> => {
  busy.value = true

  try {
    await action()
  } finally {
    busy.value = false
    confirmingRevoke.value = false
  }
}

</script>

<template>
  <UCard :ui="{ body: 'grid gap-4' }">
    <div class="flex flex-wrap items-center gap-2.5">
      <h2 class="flex-1 text-[17px] font-semibold text-highlighted">
        AI access
      </h2>
      <UBadge
        :color="entry?.hasAccess ? 'success' : 'neutral'"
        variant="subtle"
        :icon="entry?.hasAccess ? 'i-lucide-check' : undefined"
      >
        {{ entry?.hasAccess ? 'Has access' : 'No access' }}
      </UBadge>
    </div>

    <template v-if="entry?.hasAccess">
      <p
        v-if="entry.grantedBy || entry.grantedAt"
        class="-mt-2 text-[13px] text-muted"
      >
        Granted{{ entry.grantedBy ? ` by ${entry.grantedBy}` : '' }}
        <template v-if="entry.grantedAt">
          on
          <NuxtTime
            :datetime="entry.grantedAt"
            day="numeric"
            month="short"
            year="numeric"
          />
        </template>
      </p>

      <div class="grid gap-5 md:grid-cols-2">
        <div class="grid content-start gap-2.5">
          <span class="text-sm font-medium">Credits today</span>
          <div class="flex items-baseline gap-2">
            <b class="text-3xl font-semibold tabular-nums">{{ entry.credits?.remaining ?? 0 }}</b>
            <span class="text-sm text-muted">left · resets at midnight UTC</span>
          </div>
          <UProgress
            :model-value="Math.min(entry.credits?.remaining ?? 0, DAILY_CREDITS)"
            :max="DAILY_CREDITS"
          />
          <div class="flex flex-wrap items-center gap-2">
            <UInputNumber
              v-model="credits"
              :min="1"
              :max="1000"
              class="w-36"
            />
            <UButton
              icon="i-lucide-plus"
              color="secondary"
              variant="soft"
              :loading="busy"
              @click="run(() => addCredits(discordId, credits))"
            >
              Add credits for today
            </UButton>
          </div>
        </div>

        <div class="grid content-start gap-3">
          <USwitch
            :model-value="entry.canSelectModel"
            label="Can pick a model"
            description="Lets them choose the AI provider in the desktop app."
            :disabled="busy"
            @update:model-value="run(() => setCanSelectModel(discordId, !!$event))"
          />
          <UFormField
            label="Note"
            description="Only admins see this."
          >
            <div class="flex gap-2">
              <UInput
                v-model="note"
                placeholder="Why they have access"
                class="min-w-0 flex-1"
                :maxlength="500"
              />
              <UButton
                color="neutral"
                variant="subtle"
                :disabled="busy || note === (entry.note ?? '')"
                @click="run(() => saveNote(entry!, note))"
              >
                Save
              </UButton>
            </div>
          </UFormField>

          <div
            v-if="confirmingRevoke"
            class="flex flex-wrap items-center gap-2 rounded-field border border-error/40 bg-error/10 p-3 text-sm"
          >
            <span class="flex-1">Revoke AI access? Their history stays.</span>
            <UButton
              size="md"
              color="neutral"
              variant="subtle"
              @click="confirmingRevoke = false"
            >
              Cancel
            </UButton>
            <UButton
              size="md"
              color="error"
              :loading="busy"
              @click="run(() => revoke(discordId))"
            >
              Revoke
            </UButton>
          </div>
          <UButton
            v-else
            icon="i-lucide-user-minus"
            color="error"
            variant="soft"
            class="w-fit"
            @click="confirmingRevoke = true"
          >
            Revoke access
          </UButton>
        </div>
      </div>
    </template>

    <template v-else>
      <p class="-mt-2 text-[13px] text-muted">
        They can use the app normally, but not AI suggestions.
      </p>
      <UFormField
        label="Note"
        description="Only admins see this."
      >
        <UInput
          v-model="note"
          placeholder="Why they have access, for example: beta tester"
          class="w-full"
          :maxlength="500"
        />
      </UFormField>
      <UButton
        icon="i-lucide-user-plus"
        class="w-fit"
        :loading="busy"
        @click="run(() => grant(discordId, { note }))"
      >
        Grant AI access
      </UButton>
    </template>
  </UCard>
</template>
