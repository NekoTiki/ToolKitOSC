<script setup lang="ts">
import IconSelectMenu from '@renderer/components/IconSelectMenu.vue'
import AddParameterField from '@renderer/components/preset-modal/AddParameterField.vue'
import CoverageBanner from '@renderer/components/preset-modal/CoverageBanner.vue'
import ParameterRow from '@renderer/components/preset-modal/ParameterRow.vue'
import { usePresets } from '@renderer/composables/usePresets'
import type { PresetParameter } from '@vrc-osc-toolkit/shared-ui'
import _ from 'lodash'
import { ref } from 'vue'

type PresetModalModel = {
  id?: string
  name: string
  icon?: string
  parameters: PresetParameter[]
}

// Omitting `presetId` (see PresetListModal.vue's "Create from Current State") always means
// "start from whatever's currently captured", never a blank form - there's no reachable "create an
// empty preset" flow in this app, so the no-id case just folds into the same capture behavior.
const props = defineProps<{ presetId?: string }>()
const open = defineModel<boolean>('open')

const { getPreset, addPreset, updatePreset, captureParameters, coverage } = usePresets()

const buildInitialModel = (): PresetModalModel => {
  const preset = props.presetId ? getPreset(props.presetId) : undefined

  if (preset) {
    return {
      id: preset.id,
      name: preset.name,
      icon: preset.icon,
      parameters: _.cloneDeep(preset.parameters)
    }
  }

  return { name: '', icon: '', parameters: captureParameters() }
}

// Snapshotted once at mount, not kept live in sync - useOverlay mounts a fresh instance of this
// modal per open() (see usePresetModal.ts), so there's no case where `presetId` changes under an
// already-open instance.
const model = ref<PresetModalModel>(buildInitialModel())

// Re-syncs every currently-captured, non-excluded parameter into the model - adding newly-seen
// ones and refreshing values for ones already present, without dropping a parameter the user
// added by hand (see AddParameterField.vue's picker) that VRChat hasn't reported a live value for
// yet.
const captureIntoModel = (): void => {
  const captured = captureParameters()
  const byAddress = new Map(model.value.parameters.map((param) => [param.address, param]))

  captured.forEach((param) => byAddress.set(param.address, param))

  model.value.parameters = Array.from(byAddress.values())
}

const removeParameter = (address: string): void => {
  model.value.parameters = model.value.parameters.filter((param) => param.address !== address)
}

const submit = (): void => {
  if (!model.value.name.trim()) return

  if (!model.value.id) {
    addPreset(model.value.name, model.value.icon, model.value.parameters)
  } else {
    updatePreset(model.value.id, {
      name: model.value.name,
      icon: model.value.icon,
      parameters: model.value.parameters
    })
  }

  open.value = false
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

            <p
              v-if="!model.parameters.length"
              class="text-sm text-muted"
            >
              No parameters yet - use "Capture Current State" or add one manually below.
            </p>
            <!-- virtualize: a preset can hold anywhere from a handful of parameters up to
            every one an avatar declares, so this is rendered the same way the noisy-parameter
            list (ExcludedParametersModal.vue) handles a potentially long list. Row spacing comes
            from the item slot's own padding (`pb-2 last:pb-0`), not a viewport `gap` - virtualized
            rows are absolutely positioned, which a flex gap has no effect on. -->
            <UScrollArea
              v-else
              :items="model.parameters"
              virtualize
              class="h-80"
              :ui="{ item: 'pb-2 last:pb-0' }"
            >
              <template #default="{ item: parameter, index }">
                <ParameterRow
                  v-model="model.parameters[index]"
                  @remove="removeParameter(parameter.address)"
                />
              </template>
            </UScrollArea>
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
