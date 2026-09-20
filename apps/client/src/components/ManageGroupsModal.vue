<script setup lang="ts">
import { useAreYouSureModal } from '@renderer/composables/useAreYouSureModal'
import { useControlGroupOpenState } from '@renderer/composables/useControlGroupOpenState'
import { useControls } from '@renderer/composables/useControls'
import { computed, ref, watch } from 'vue'

const open = defineModel<boolean>('open')

const { controls, deleteGroups } = useControls()
const { setOpen } = useControlGroupOpenState()
const { openModal: ausOpenModal } = useAreYouSureModal()
const toast = useToast()

const selectedIds = ref<Set<string>>(new Set())

// Reset on close, not just after a successful delete below - useOverlay() may or may not keep this
// component mounted across opens (see AiControlsModal.vue's identical note on this), so this is
// what guarantees re-opening always starts from a clean selection either way.
watch(open, (isOpen) => {
  if (!isOpen) selectedIds.value = new Set()
})

const allSelected = computed(
  () => controls.value.length > 0 && selectedIds.value.size === controls.value.length
)

function toggleSelected(groupId: string): void {
  const next = new Set(selectedIds.value)

  if (next.has(groupId)) next.delete(groupId)
  else next.add(groupId)

  selectedIds.value = next
}

function selectAll(): void {
  selectedIds.value = allSelected.value ? new Set() : new Set(controls.value.map((g) => g.id))
}

function collapseSelected(): void {
  selectedIds.value.forEach((id) => setOpen(id, false))
}

function expandSelected(): void {
  selectedIds.value.forEach((id) => setOpen(id, true))
}

async function deleteSelected(): Promise<void> {
  const count = selectedIds.value.size

  if (!count) return

  const result = await ausOpenModal({
    title: 'Delete Control Groups',
    message: `Are you sure you want to delete ${count} control group${count === 1 ? '' : 's'}?\n\nThis action cannot be undone.`,
    confirmText: 'Delete'
  })

  if (!result) return

  deleteGroups([...selectedIds.value])
  toast.add({ title: `Deleted ${count} group${count === 1 ? '' : 's'}`, icon: 'i-lucide-check', color: 'success' })
  selectedIds.value = new Set()
}
</script>

<template>
  <UModal
    v-model:open="open"
    title="Manage Control Groups"
    description="Select groups to collapse, expand, or delete in bulk."
    :ui="{ content: 'max-w-lg', body: 'flex flex-col gap-3' }"
  >
    <template #body>
      <div
        v-if="!controls.length"
        class="py-6 text-center text-sm text-muted"
      >
        No control groups yet.
      </div>

      <template v-else>
        <div class="flex items-center justify-between gap-2">
          <UCheckbox
            :model-value="allSelected"
            label="Select All"
            @update:model-value="selectAll"
          />
          <span class="text-xs text-muted">{{ selectedIds.size }} / {{ controls.length }} selected</span>
        </div>

        <div class="flex max-h-80 flex-col divide-y divide-default overflow-y-auto rounded-md border border-default">
          <label
            v-for="group in controls"
            :key="group.id"
            class="flex cursor-pointer items-center gap-2 px-3 py-2 hover:bg-elevated"
          >
            <UCheckbox
              :model-value="selectedIds.has(group.id)"
              @update:model-value="toggleSelected(group.id)"
            />
            <span class="truncate">{{ group.name }}</span>
            <span class="ml-auto shrink-0 text-xs text-muted">
              {{ group.controls.length }} control{{ group.controls.length === 1 ? '' : 's' }}
            </span>
          </label>
        </div>

        <div class="flex flex-wrap justify-end gap-2">
          <UButton
            color="neutral"
            variant="soft"
            icon="i-lucide-chevrons-down-up"
            :disabled="!selectedIds.size"
            @click="collapseSelected"
          >
            Collapse
          </UButton>
          <UButton
            color="neutral"
            variant="soft"
            icon="i-lucide-chevrons-up-down"
            :disabled="!selectedIds.size"
            @click="expandSelected"
          >
            Expand
          </UButton>
          <UButton
            color="error"
            variant="soft"
            icon="i-lucide-trash-2"
            :disabled="!selectedIds.size"
            @click="deleteSelected"
          >
            Delete
          </UButton>
        </div>
      </template>
    </template>
  </UModal>
</template>

<style scoped></style>
