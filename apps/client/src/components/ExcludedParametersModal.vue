<script setup lang="ts">
import type { SelectMenuItem } from '@nuxt/ui/components/SelectMenu.vue'
import { usePresets } from '@renderer/composables/usePresets'
import { isNoisyAddress } from '@renderer/utils/addressFilter'
import { computed, ref } from 'vue'

const open = defineModel<boolean>('open')
const { filterableParameters, setExcluded } = usePresets()

const selected = ref<string>()

// Split three ways: what's already custom-excluded (editable here), what's hidden automatically
// by the hardcoded noisy-address denylist (informational only - re-including one of these isn't
// this list's job, `AddParameterField.vue`'s own "show all" toggle is how those get in), and what's
// left to offer in the "add exclusion" picker below.
const excluded = computed(() => filterableParameters.value.filter((param) => param.excluded))
const noisy = computed(() =>
  filterableParameters.value.filter((param) => !param.excluded && isNoisyAddress(param.name))
)

const items = computed<SelectMenuItem[]>(() =>
  filterableParameters.value
    .filter((param) => !param.excluded && !isNoisyAddress(param.name))
    .map((param) => ({ label: param.name, value: param.address }))
)

const addExclusion = (): void => {
  if (!selected.value) return

  setExcluded(selected.value, true)
  selected.value = undefined
}
</script>

<template>
  <UModal
    v-model:open="open"
    :ui="{ content: 'max-w-lg', body: 'flex flex-col gap-4' }"
    title="Excluded Parameters"
    description="Parameters excluded here are hidden from this avatar's preset capture and parameter picker, avatar-wide."
  >
    <template #body>
      <UFormField label="Exclude a parameter">
        <div class="flex gap-2">
          <USelectMenu
            v-model="selected"
            :items="items"
            value-key="value"
            placeholder="Select a parameter"
            class="w-full grow"
            virtualize
          />
          <UButton
            icon="i-lucide-plus"
            :disabled="!selected"
            @click="addExclusion"
          />
        </div>
      </UFormField>

      <div class="flex flex-col gap-2">
        <span class="text-sm font-medium">
          {{ excluded.length }} custom exclusion(s)
        </span>
        <p
          v-if="!excluded.length"
          class="text-sm text-muted"
        >
          No custom exclusions for this avatar yet.
        </p>
        <UScrollArea
          v-else
          :items="excluded"
          class="h-60"
          :ui="{ item: 'pb-2 last:pb-0' }"
        >
          <template #default="{ item: param }">
            <UCard :ui="{ body: 'p-2 sm:p-2 flex justify-between items-center gap-2' }">
              <div class="min-w-0">
                <p class="truncate text-sm">
                  {{ param.name }}
                </p>
                <p class="truncate text-xs text-muted">
                  {{ param.address }}
                </p>
              </div>
              <UButton
                icon="i-lucide-eye"
                color="neutral"
                variant="subtle"
                size="sm"
                @click="setExcluded(param.address, false)"
              >
                Re-include
              </UButton>
            </UCard>
          </template>
        </UScrollArea>
      </div>

      <!-- Fixed h-40 (not max-h + overflow on a plain div): UCollapsible measures its content's
      natural height for the open/close animation, which doesn't play well with a clamped/scrolling
      child - the content could end up rendered but effectively zero-height. A UScrollArea has an
      explicit height of its own, so the collapsible always has something concrete to measure.
      virtualize: this list can get long (every PhysBone-generated parameter across every bone), so
      it's rendered the same way AddressSelect.vue/AddParameterField.vue's pickers already handle a
      big parameter list. -->
      <UCollapsible v-if="noisy.length">
        <UButton
          :label="`${noisy.length} hidden automatically (noisy address patterns)`"
          color="neutral"
          variant="ghost"
          trailing-icon="i-lucide-chevron-down"
          block
        />

        <template #content>
          <UScrollArea
            :items="noisy"
            virtualize
            class="mt-2 h-40"
          >
            <template #default="{ item: param }">
              <span class="block truncate text-xs text-muted">{{ param.name }}</span>
            </template>
          </UScrollArea>
        </template>
      </UCollapsible>
    </template>
  </UModal>
</template>

<style scoped></style>
