<script setup lang="ts">
interface AccessEntry {
  discordId: string
  username: string | null
  displayName: string | null
  avatarUrl: string | null
  lastSeenAt: string
  hasAccess: boolean
  canSelectModel: boolean
  grantedBy: string | null
  grantedAt: string | null
  note: string | null
  // One overall pool per account per day (see the server's rateLimit.ts), not per-provider - null
  // for a user with no access yet.
  credits: { remaining: number } | null
}

const { adminFetch } = useAdminApi()
const toast = useToast()

const users = ref<AccessEntry[]>([])
const loading = ref(true)
const filter = ref('')

const filteredUsers = computed(() => {
  const q = filter.value.trim().toLowerCase()

  if (!q) return users.value

  return users.value.filter(
    (u) =>
      u.discordId.includes(q) ||
      u.username?.toLowerCase().includes(q) ||
      u.displayName?.toLowerCase().includes(q)
  )
})

async function load(): Promise<void> {
  loading.value = true

  try {
    const res = await adminFetch<{ users: AccessEntry[] }>('/api/admin/users')

    users.value = res.users
  } finally {
    loading.value = false
  }
}

onMounted(load)

async function grantAccess(discordId: string, canSelectModel: boolean): Promise<void> {
  if (!discordId.trim()) return

  await adminFetch('/api/admin/users', {
    method: 'POST',
    body: { discordId: discordId.trim(), canSelectModel }
  })
  toast.add({ title: 'Access granted', icon: 'i-lucide-check', color: 'success' })
  await load()
}

async function revokeAccess(discordId: string): Promise<void> {
  await adminFetch(`/api/admin/users/${discordId}`, { method: 'DELETE' })
  toast.add({ title: 'Access revoked', icon: 'i-lucide-x', color: 'neutral' })
  await load()
}

async function toggleCanSelectModel(user: AccessEntry): Promise<void> {
  await adminFetch(`/api/admin/users/${user.discordId}`, {
    method: 'PATCH',
    body: { canSelectModel: !user.canSelectModel }
  })
  await load()
}

const creditPopoverFor = ref<string | null>(null)
const creditAmount = ref(5)

function openCreditPopover(discordId: string): void {
  creditPopoverFor.value = creditPopoverFor.value === discordId ? null : discordId
  creditAmount.value = 5
}

async function addCredits(discordId: string): Promise<void> {
  if (creditAmount.value < 1) return

  await adminFetch(`/api/admin/users/${discordId}/credits`, {
    method: 'POST',
    body: { amount: creditAmount.value }
  })
  toast.add({ title: `Added ${creditAmount.value} credits`, icon: 'i-lucide-check', color: 'success' })
  creditPopoverFor.value = null
  await load()
}
</script>

<template>
  <div class="flex flex-col gap-4">
    <UCard>
      <div class="mb-3 flex items-center justify-between gap-2">
        <p class="text-sm font-medium">
          Every known user ({{ filteredUsers.length }})
        </p>
        <UInput
          v-model="filter"
          icon="i-lucide-search"
          placeholder="Filter by name or id"
          class="w-64"
        />
      </div>

      <div
        v-if="loading"
        class="py-8 text-center text-sm text-muted"
      >
        Loading…
      </div>

      <div
        v-else
        class="flex flex-col divide-y divide-default"
      >
        <div
          v-for="user in filteredUsers"
          :key="user.discordId"
          class="flex flex-col gap-2 py-3"
        >
          <div class="flex flex-wrap items-center gap-3">
            <UAvatar
              :src="user.avatarUrl ?? undefined"
              :alt="user.displayName ?? user.discordId"
            />
            <div class="min-w-0">
              <p class="truncate font-medium">
                {{ user.displayName || user.username || 'Unknown' }}
              </p>
              <p class="truncate text-xs text-muted">
                {{ user.discordId }} · last seen {{ new Date(user.lastSeenAt).toLocaleString() }}
              </p>
            </div>

            <div class="ml-auto flex flex-wrap items-center gap-2">
              <template v-if="user.hasAccess">
                <UBadge
                  v-if="user.credits"
                  color="neutral"
                  variant="subtle"
                >
                  Credits: {{ user.credits.remaining }}
                </UBadge>
                <UButton
                  size="xs"
                  variant="soft"
                  icon="i-lucide-plus"
                  @click="openCreditPopover(user.discordId)"
                >
                  Credits
                </UButton>
                <USwitch
                  :model-value="user.canSelectModel"
                  label="Can select model"
                  @update:model-value="toggleCanSelectModel(user)"
                />
                <UButton
                  size="xs"
                  color="error"
                  variant="soft"
                  icon="i-lucide-user-minus"
                  @click="revokeAccess(user.discordId)"
                >
                  Revoke
                </UButton>
              </template>
              <UButton
                v-else
                size="xs"
                icon="i-lucide-user-plus"
                @click="grantAccess(user.discordId, false)"
              >
                Grant access
              </UButton>
            </div>
          </div>

          <div
            v-if="creditPopoverFor === user.discordId"
            class="flex flex-wrap items-center gap-2 rounded-md bg-elevated p-2"
          >
            <UInputNumber
              v-model="creditAmount"
              :min="1"
              :max="1000"
              class="w-28"
            />
            <UButton
              size="xs"
              @click="addCredits(user.discordId)"
            >
              Add
            </UButton>
          </div>
        </div>
      </div>
    </UCard>
  </div>
</template>
