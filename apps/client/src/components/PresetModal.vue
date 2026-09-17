<script setup lang="ts">
import IconSelectMenu from '@renderer/components/IconSelectMenu.vue'
import AddParameterField from '@renderer/components/preset-modal/AddParameterField.vue'
import CoverageBanner from '@renderer/components/preset-modal/CoverageBanner.vue'
import ParameterRow from '@renderer/components/preset-modal/ParameterRow.vue'
import { usePresetModal } from '@renderer/composables/usePresetModal'
import { usePresets } from '@renderer/composables/usePresets'

const { model, open, captureIntoModel, submit } = usePresetModal()
const { coverage } = usePresets()

const removeParameter = (address: string): void => {
  model.value.parameters = model.value.parameters.filter((param) => param.address !== address)
}
</script>

<template>
  <UModal
    v-model:open="open"
    :ui="{ content: 'max-w-2xl' }"
    :dismissible="false"
  >
    <template #content>
      <UCard :ui="{ root: 'overflow-auto', body: 'flex flex-col gap-4 max-h-full' }">
        <UForm class="flex h-min max-h-full grow flex-col gap-4">
          <UFormField
            label="Name"
            required
          >
            <UInput
              v-model="model.name"
              class="w-full"
            />
          </UFormField>
          <UFormField label="Icon">
            <IconSelectMenu
              v-model="model.icon"
              class="w-full"
              @keyup.backspace="model.icon = ''"
            />
          </UFormField>

          <CoverageBanner
            :captured="coverage.captured"
            :total="coverage.total"
          />

          <UFormField
            :ui="{ container: 'grid gap-2' }"
            label="Parameters"
          >
            <div class="flex items-center justify-between gap-2">
              <span class="text-sm text-muted">{{ model.parameters.length }} parameter(s) in this preset</span>
              <UButton
                icon="i-lucide-refresh-cw"
                color="neutral"
                variant="subtle"
                size="sm"
                @click="captureIntoModel"
              >
                Capture Current State
              </UButton>
            </div>

            <div class="flex max-h-80 flex-col gap-2 overflow-y-auto">
              <ParameterRow
                v-for="(parameter, index) in model.parameters"
                :key="parameter.address"
                v-model="model.parameters[index]"
                @remove="removeParameter(parameter.address)"
              />

              <p
                v-if="!model.parameters.length"
                class="text-sm text-muted"
              >
                No parameters yet - use "Capture Current State" or add one manually below.
              </p>
            </div>
          </UFormField>

          <AddParameterField v-model:parameters="model.parameters" />
        </UForm>
        <div class="flex justify-end gap-2">
          <UButton
            variant="subtle"
            color="neutral"
            @click="open = false"
          >
            Cancel
          </UButton>
          <UButton @click="submit">
            Save
          </UButton>
        </div>
      </UCard>
    </template>
  </UModal>
</template>

<style scoped></style>
