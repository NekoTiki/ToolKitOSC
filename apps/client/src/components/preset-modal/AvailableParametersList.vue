<script setup lang="ts">
import AddressListToggle from '@renderer/components/control-modal/AddressListToggle.vue'
import type { FilterableParameter } from '@renderer/composables/usePresets'
import { usePresets } from '@renderer/composables/usePresets'
import { isNoisyAddress } from '@renderer/utils/addressFilter'
import type { PresetParameter } from '@vrc-osc-toolkit/shared-ui'
import { computed, ref } from 'vue'

const parameters = defineModel<PresetParameter[]>('parameters', { required: true })

const { filterableParameters, setExcluded } = usePresets()

const showAll = ref(false)

const includedAddresses = computed(() => new Set(parameters.value.map((p) => p.address)))

// virtualize below: an avatar's full parameter list can run into the hundreds, same reasoning as
// every other potentially-long list in this app (see pages/ParametersPage.vue).
const availableParameters = computed<FilterableParameter[]>(() =>
  filterableParameters.value
    .filter((param) => !includedAddresses.value.has(param.address))
    .filter((param) => showAll.value || (!param.excluded && !isNoisyAddress(param.name)))
)

const add = (param: FilterableParameter): void => {
  parameters.value = [
    ...parameters.value,
    { address: param.address, name: param.name, kind: param.kind, value: param.value ?? false }
  ]
}
</script>

<template>
  <div class="flex flex-col gap-2">
    <AddressListToggle v-model="showAll" />

    <p
      v-if="!availableParameters.length"
      class="text-sm text-muted"
    >
      Nothing left to add{{ showAll ? '' : ' - try "Show raw address list" for noisy/excluded parameters' }}.
    </p>

    <UScrollArea
      v-else
      :items="availableParameters"
      virtualize
      class="h-80"
      :ui="{ item: 'pb-2 last:pb-0' }"
    >
      <template #default="{ item: parameter }">
        <div class="flex items-center gap-2">
          <div class="min-w-0 grow">
            <p class="truncate text-sm font-medium">
              {{ parameter.name }}
            </p>
            <p class="truncate text-xs text-muted">
              {{ parameter.address }}
              <span v-if="!parameter.captured">- not seen yet</span>
            </p>
          </div>
          <UTooltip>
            <UButton
              :icon="parameter.excluded ? 'i-lucide-eye' : 'i-lucide-eye-off'"
              color="neutral"
              variant="outline"
              size="sm"
              @click="setExcluded(parameter.address, !parameter.excluded)"
            />

            <template #content>
              {{
                parameter.excluded
                  ? 'Re-include this address avatar-wide'
                  : 'Exclude this address avatar-wide from future presets'
              }}
            </template>
          </UTooltip>
          <UButton
            icon="i-lucide-plus"
            color="primary"
            variant="subtle"
            size="sm"
            @click="add(parameter)"
          >
            Add
          </UButton>
        </div>
      </template>
    </UScrollArea>
  </div>
</template>

<style scoped></style>
