<script setup lang="ts">
import { useAreYouSureModal } from '@renderer/composables/useAreYouSureModal'
import { useExcludedParametersModal } from '@renderer/composables/useExcludedParametersModal'
import { usePresetModal } from '@renderer/composables/usePresetModal'
import { usePresets } from '@renderer/composables/usePresets'

const { listOpen, openModal, openCreateFromCurrentState } = usePresetModal()
const { presets, deletePreset, applyPreset } = usePresets()
const { openModal: ausOpenModal } = useAreYouSureModal()
const { open: excludedParametersOpen } = useExcludedParametersModal()

const handleManageExcluded = (): void => {
  excludedParametersOpen.value = true
  listOpen.value = false
}

const toast = useToast()

const handleCreate = (): void => {
  openCreateFromCurrentState()
  listOpen.value = false
}

const handleEdit = (presetId: string): void => {
  openModal(presetId)
  listOpen.value = false
}

const handleApply = (presetId: string): void => {
  applyPreset(presetId)
  toast.add({ title: 'Preset applied', icon: 'i-lucide-check', color: 'success' })
}

const handleRemove = async (presetId: string): Promise<void> => {
  const preset = presets.value.find((p) => p.id === presetId)
  if (!preset) return

  const result = await ausOpenModal({
    title: 'Delete Preset',
    message: `Are you sure you want to delete the preset "${preset.name}"?`,
    confirmText: 'Delete'
  })

  if (!result) return

  deletePreset(presetId)
}
</script>

<template>
  <UModal
    v-model:open="listOpen"
    :ui="{ body: 'flex flex-col max-w-lg gap-4' }"
    title="Presets"
  >
    <template #body>
      <div class="flex items-center justify-between gap-2">
        <h3 v-if="!presets.length">
          No presets found for this avatar.
        </h3>
        <span
          v-else
          class="text-sm text-muted"
        >{{ presets.length }} preset(s)</span>
        <div class="flex shrink-0 gap-2">
          <UTooltip>
            <UButton
              icon="i-lucide-eye-off"
              color="neutral"
              variant="subtle"
              @click="handleManageExcluded"
            />

            <template #content>
              Manage Excluded Parameters
            </template>
          </UTooltip>
          <UButton
            color="primary"
            variant="subtle"
            icon="i-lucide-camera"
            @click="handleCreate"
          >
            Create from Current State
          </UButton>
        </div>
      </div>
      <UCard
        v-for="preset in presets"
        :key="preset.id"
        :ui="{ body: 'p-2 sm:p-2 flex justify-between items-center gap-2' }"
      >
        <div class="flex min-w-0 items-center gap-2">
          <UIcon
            v-if="preset.icon"
            :name="preset.icon"
          />
          <div class="min-w-0">
            <p class="truncate">
              {{ preset.name }}
            </p>
            <p class="truncate text-xs text-muted">
              {{ preset.parameters.length }} parameter(s) - updated {{ new Date(preset.updatedAt).toLocaleString() }}
            </p>
          </div>
        </div>
        <div class="flex shrink-0 gap-2">
          <UTooltip>
            <UButton
              icon="i-lucide-play"
              color="primary"
              variant="subtle"
              @click="handleApply(preset.id)"
            />

            <template #content>
              Apply this preset now
            </template>
          </UTooltip>
          <UButton
            icon="i-lucide-pen"
            color="secondary"
            variant="subtle"
            @click="handleEdit(preset.id)"
          />
          <UButton
            icon="i-lucide-trash"
            color="error"
            variant="subtle"
            @click="handleRemove(preset.id)"
          />
        </div>
      </UCard>
    </template>
  </UModal>
</template>

<style scoped></style>
