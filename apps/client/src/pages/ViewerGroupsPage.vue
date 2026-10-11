<script setup lang="ts">
import EmptyState from '@renderer/components/ui/EmptyState.vue'
import FormSection from '@renderer/components/ui/FormSection.vue'
import { useAreYouSureModal } from '@renderer/composables/useAreYouSureModal'
import { useLiveQuery } from '@renderer/composables/useLiveQuery'
import type { ViewerGroup } from '@renderer/composables/useViewerGroups'
import { GROUP_COLORS, useViewerGroups } from '@renderer/composables/useViewerGroups'
import type { Client } from '@renderer/db/clients.db'
import { db as clientsDb } from '@renderer/db/clients.db'
import { computed, nextTick } from 'vue'

// Viewer groups (Friends, Mods...): name and color each one, see who's in it, delete it. Viewers are
// added from their own page; what a group gets is set per profile (Profiles > Editing for).
const toast = useToast()
const { viewerGroups, addGroup, updateGroup, deleteGroup, membershipsOf } = useViewerGroups()
const { openModal: confirm } = useAreYouSureModal()

const members = useLiveQuery(
  () => null,
  () => clientsDb.clients.filter((client) => !!client.groups?.length).toArray(),
  [] as Client[]
)

const membersOf = (group: ViewerGroup): Client[] =>
  members.value
    .filter((client) => membershipsOf(client).some((membership) => membership.id === group.id))
    .sort((a, b) => a.displayName.localeCompare(b.displayName))

const meta = computed(() => `${viewerGroups.value.length} ${viewerGroups.value.length === 1 ? 'group' : 'groups'}`)

const create = async (): Promise<void> => {
  const group = addGroup(`Group ${viewerGroups.value.length + 1}`)

  await nextTick()
  const input = document.getElementById(`group-name-${group.id}`) as HTMLInputElement | null

  input?.focus()
  input?.select()
}

const rename = (group: ViewerGroup, value: string): void => {
  const name = value.trim()

  if (name && name !== group.name) updateGroup(group.id, { name })
}

const remove = async (group: ViewerGroup): Promise<void> => {
  const count = membersOf(group).length
  const ok = await confirm({
    title: `Delete ${group.name}?`,
    message: `${count ? `Its ${count} ${count === 1 ? 'viewer goes' : 'viewers go'} back to what everyone gets. ` : ''}Profiles stop giving it anything extra.`,
    confirmText: 'Delete'
  })

  if (!ok) return

  await deleteGroup(group.id)
  toast.add({ title: `${group.name} deleted`, icon: 'i-lucide-trash-2' })
}
</script>

<template>
  <PageLayout>
    <template #header>
      <PageHeader
        title="Viewer groups"
        :meta="meta"
        :crumbs="[{ label: 'Viewers', to: '/viewers' }, { label: 'Groups' }]"
      >
        <template #actions>
          <UButton
            icon="i-lucide-plus"
            @click="create"
          >
            New group
          </UButton>
        </template>
      </PageHeader>
    </template>

    <div
      v-if="viewerGroups.length"
      class="grid max-w-4xl grid-cols-1 gap-3.5"
    >
      <p class="px-0.5 text-[13px] leading-snug text-muted">
        Add viewers to a group from their page in Viewers. Then, in each profile, pick the group under <b>Editing for</b> to unlock controls or raise limits for it. Only you can put people in a group.
      </p>

      <FormSection
        v-for="group in viewerGroups"
        :key="group.id"
      >
        <div class="flex flex-wrap items-center gap-3">
          <span
            class="size-4 shrink-0 rounded-full"
            :style="{ backgroundColor: group.color }"
          />
          <UInput
            :id="`group-name-${group.id}`"
            :model-value="group.name"
            :aria-label="`${group.name} name`"
            class="min-w-40 flex-1"
            @change="rename(group, ($event.target as HTMLInputElement).value)"
            @keydown.enter="($event.target as HTMLInputElement).blur()"
          />
          <UButton
            icon="i-lucide-trash-2"
            color="error"
            variant="soft"
            @click="remove(group)"
          >
            Delete
          </UButton>
        </div>

        <div
          class="flex flex-wrap gap-2"
          role="radiogroup"
          :aria-label="`${group.name} color`"
        >
          <button
            v-for="color in GROUP_COLORS"
            :key="color"
            type="button"
            role="radio"
            :aria-checked="group.color === color"
            :aria-label="color"
            class="grid size-9 cursor-pointer place-items-center rounded-full border-2 transition-transform hover:scale-110"
            :class="group.color === color ? 'border-(--ui-text-highlighted)' : 'border-transparent'"
            @click="updateGroup(group.id, { color })"
          >
            <span
              class="size-6 rounded-full"
              :style="{ backgroundColor: color }"
            />
          </button>
        </div>

        <div class="flex flex-wrap items-center gap-2">
          <RouterLink
            v-for="client in membersOf(group)"
            :key="client.id"
            :to="`/viewers/${client.id}`"
            class="flex items-center gap-2 rounded-full bg-(--aurora-well) py-1 pr-3 pl-1 text-sm transition-colors hover:bg-(--aurora-glass)"
          >
            <UAvatar
              :src="client.avatar"
              :alt="client.displayName"
              size="xs"
            />
            {{ client.displayName }}
          </RouterLink>
          <span
            v-if="!membersOf(group).length"
            class="text-sm text-muted"
          >Nobody in this group yet.</span>
        </div>
      </FormSection>
    </div>

    <EmptyState
      v-else
      icon="i-lucide-users-round"
      title="No viewer groups yet"
      description="Make groups like Friends or Mods, put viewers in them, and let each profile give a group more than everyone else gets."
    >
      <template #actions>
        <UButton
          icon="i-lucide-plus"
          @click="create"
        >
          New group
        </UButton>
      </template>
    </EmptyState>
  </PageLayout>
</template>
