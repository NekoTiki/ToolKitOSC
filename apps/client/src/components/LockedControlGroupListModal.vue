<script setup lang="ts">
import { useAreYouSureModal } from '@renderer/composables/useAreYouSureModal'
import { useLockedControls } from '@renderer/composables/useLockedControls'
import { useLockedControlsModal } from '@renderer/composables/useLockedControlsModal'

const { listOpen, openModal } = useLockedControlsModal()
const { lockedControlGroups, removeLockedControlGroup } = useLockedControls()
const { openModal: ausOpenModal } = useAreYouSureModal()

const handleCreate = (): void => {
  openModal()
  listOpen.value = false
}

const handleEdit = (groupId: string): void => {
  openModal(groupId)
  listOpen.value = false
}

const handleRemove = async (groupId: string): Promise<void> => {
  const group = lockedControlGroups.value.find((group) => group.id === groupId)
  if (!group) return

  const result = await ausOpenModal({
    title: 'Remove Locked Control Group',
    message: `Are you sure you want to remove the locked control group "${group.name}"?`,
    confirmText: 'Remove'
  })

  if (!result) return

  removeLockedControlGroup(groupId)
}
</script>

<template>
  <UModal
    v-model:open="listOpen"
    :ui="{ body: 'flex flex-col max-w-lg gap-4' }"
    title="Locked Control Groups"
  >
    <template #body>
      <div
        v-if="!lockedControlGroups.length"
        class="flex items-center justify-between gap-2"
      >
        <h3>No locked control groups found.</h3>
        <UButton
          color="primary"
          variant="subtle"
          @click="handleCreate"
        >
          Create New
        </UButton>
      </div>
      <UCard
        v-for="group in lockedControlGroups"
        :key="group.id"
        :ui="{ body: 'p-2 sm:p-2 flex justify-between items-center gap-2' }"
      >
        <span>{{ group.name }}</span>
        <div class="flex gap-2">
          <UButton
            icon="i-lucide-pen"
            color="secondary"
            variant="subtle"
            @click="handleEdit(group.id)"
          />
          <UButton
            icon="i-lucide-trash"
            color="error"
            variant="subtle"
            @click="handleRemove(group.id)"
          />
        </div>
      </UCard>
    </template>
  </UModal>
</template>

<style scoped></style>
