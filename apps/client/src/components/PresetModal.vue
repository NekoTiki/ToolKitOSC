<script setup lang="ts">
import IconSelectMenu from '@renderer/components/IconSelectMenu.vue'
import AvailableParametersList from '@renderer/components/preset-modal/AvailableParametersList.vue'
import CoverageBanner from '@renderer/components/preset-modal/CoverageBanner.vue'
import ParameterRow from '@renderer/components/preset-modal/ParameterRow.vue'
import { usePresets } from '@renderer/composables/usePresets'
import { api } from '@renderer/lib/tauri-bridge'
import type { PresetParameter } from '@vrc-osc-toolkit/shared-ui'
import _ from 'lodash'
import { computed, onMounted, ref } from 'vue'

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

const { getPreset, addPreset, updatePreset, captureParameters, coverage, includedParameters } =
  usePresets()

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

// Only for a brand new preset (never when editing an existing one, which keeps its saved
// parameters until the user explicitly asks to re-capture) - fetches every parameter's actual
// current value from VRChat over OSCQuery instead of relying on whatever's happened to be pushed
// so far, then folds the result into the model the same way the "Capture Current State" button
// does. Best-effort: if OSCQuery can't reach VRChat right now (VPN, firewall, VRChat not running),
// this just leaves the model at whatever was already captured via push.
const pullingInitial = ref(false)

onMounted(async () => {
  if (props.presetId) return

  pullingInitial.value = true
  try {
    await api.forcePullParameters(false)
    captureIntoModel()
  } catch {
    // Best-effort, see comment above - nothing to surface here beyond the coverage banner already
    // reflecting whatever ended up captured.
  } finally {
    pullingInitial.value = false
  }
})

const removeParameter = (address: string): void => {
  model.value.parameters = model.value.parameters.filter((param) => param.address !== address)
}

// Counts against `includedParameters` (the default noisy/excluded filter, no "show all") so the
// tab label matches what AvailableParametersList.vue shows before the user ever touches its own
// "show all" toggle.
const availableCount = computed(() => {
  const included = new Set(model.value.parameters.map((param) => param.address))

  return includedParameters.value.filter((param) => !included.has(param.address)).length
})

const parameterTabs = computed(() => [
  { label: `In Preset (${model.value.parameters.length})`, icon: 'i-lucide-list-checks', slot: 'inPreset' },
  { label: `Available (${availableCount.value})`, icon: 'i-lucide-plus-circle', slot: 'available' }
])

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

          <div
            v-if="pullingInitial"
            class="flex items-center gap-2 text-sm text-muted"
          >
            <UIcon
              name="i-lucide-loader-2"
              class="animate-spin"
            />
            Fetching current parameter values from VRChat...
          </div>
          <CoverageBanner
            v-else
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

            <UTabs
              :items="parameterTabs"
              :ui="{ content: 'pt-2' }"
            >
              <template #inPreset>
                <p
                  v-if="!model.parameters.length"
                  class="text-sm text-muted"
                >
                  No parameters yet - use "Capture Current State" or add one from the "Available" tab.
                </p>
                <!-- virtualize: a preset can hold anywhere from a handful of parameters up to
                every one an avatar declares, so this is rendered the same way the noisy-parameter
                list (ExcludedParametersModal.vue) handles a potentially long list. Row spacing
                comes from the item slot's own padding (`pb-2 last:pb-0`), not a viewport `gap` -
                virtualized rows are absolutely positioned, which a flex gap has no effect on. -->
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
              </template>

              <template #available>
                <AvailableParametersList v-model:parameters="model.parameters" />
              </template>
            </UTabs>
          </UFormField>
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
