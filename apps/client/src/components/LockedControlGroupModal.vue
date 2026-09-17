<script setup lang="ts">
import { useControls } from '@renderer/composables/useControls'
import { getUUID } from '@renderer/composables/useLockedControlGroupModal'
import type { LockedControlGroup } from '@renderer/composables/useLockedControls'
import { useLockedControls } from '@renderer/composables/useLockedControls'
import type { ControlType } from '@vrc-osc-toolkit/shared-ui'
import _ from 'lodash'
import { ref } from 'vue'

type LockedControlGroupData = Omit<LockedControlGroup, 'id'> &
  Partial<Pick<LockedControlGroup, 'id'>>

const defaultGroup = (): LockedControlGroupData => ({ name: '', lockedControls: {} })

const props = defineProps<{ groupId?: string }>()
const open = defineModel<boolean>('open')

const { controls } = useControls()
const { getLockedControlGroup, addLockedControlGroup, updateLockedControlGroup } =
  useLockedControls()

// Snapshotted once at mount, not kept live in sync - useOverlay mounts a fresh instance per open()
// (see useLockedControlGroupModal.ts), so there's no case where `groupId` changes under an
// already-open instance.
const model = ref<LockedControlGroupData>(
  props.groupId ? (_.cloneDeep(getLockedControlGroup(props.groupId)) ?? defaultGroup()) : defaultGroup()
)

const getIcon = (control: ControlType): string | undefined => {
  if (control.type === 'open-shock-shocker') {
    if (control.mode === 'Shock') return 'material-symbols:electric-bolt'
    return 'lucide:waves'
  }

  return control.icon
}

const submit = (): void => {
  const modelData = model.value

  if (!modelData.id) {
    modelData.id = getUUID()
    addLockedControlGroup(model.value)
  } else updateLockedControlGroup(modelData.id, model.value)

  open.value = false
}
</script>

<template>
  <UModal
    v-model:open="open"
    :ui="{ content: 'max-w-2xl' }"
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
          <UFormField
            :ui="{ container: 'grid grid-cols-2 gap-4 pt-4' }"
            label="Available Controls"
            required
          >
            <UCard
              v-for="controlGroup in controls"
              :key="controlGroup.id"
              :ui="{ body: 'flex flex-col gap-2 p-4 sm:p-4' }"
            >
              <div class="mb-2 flex items-center justify-between">
                <h3 class="text-xs font-semibold">
                  {{ controlGroup.name }}
                </h3>
                <USwitch
                  :model-value="
                    controlGroup.controls.every((control) => !model.lockedControls[control.id])
                  "
                  @update:model-value="
                    controlGroup.controls.forEach((control) => {
                      model.lockedControls[control.id] = !$event
                    })
                  "
                />
              </div>

              <div
                v-for="control in controlGroup.controls"
                :key="control.id"
                class="flex gap-2"
              >
                <USwitch
                  :model-value="!model.lockedControls[control.id]"
                  @update:model-value="
                    model.lockedControls[control.id] = !model.lockedControls[control.id]
                  "
                >
                  <template #label>
                    <div
                      class="flex items-center"
                      @click="model.lockedControls[control.id] = !model.lockedControls[control.id]"
                    >
                      <UIcon
                        v-if="getIcon(control)"
                        :name="getIcon(control) as string"
                        class="mr-2"
                      />
                      {{ control.name }}
                      <UPopover
                        mode="hover"
                        :content="{ align: 'center', side: 'right' }"
                      >
                        <UIcon
                          name="i-lucide-info"
                          class="ml-2 text-muted"
                        />

                        <template #content>
                          <Control
                            :control="control"
                            class="w-55"
                          />
                        </template>
                      </UPopover>
                    </div>
                  </template>
                </USwitch>
              </div>
            </UCard>
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
