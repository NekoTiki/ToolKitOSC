<script setup lang="ts">
import type { DropdownMenuItem } from '@nuxt/ui/components/DropdownMenu.vue'
import { useAreYouSureModal } from '@renderer/composables/useAreYouSureModal'
import { useClientsDb } from '@renderer/composables/useClientsDb'
import type { Client } from '@renderer/db/clients.db'
import type { Command } from '@renderer/db/commands.db'
import { db } from '@renderer/db/commands.db'
import type { CommandWithoutIds, ControlGroup } from '@vrc-osc-toolkit/shared-ui'
import { Control } from '@vrc-osc-toolkit/shared-ui'
import { from, useObservable } from '@vueuse/rxjs'
import { liveQuery } from 'dexie'
import { computed, ref, watch } from 'vue'
import { VueDraggable } from 'vue-draggable-plus'

import { useControlModal } from '../../composables/useControlModal'
import { useControls } from '../../composables/useControls'

const props = defineProps<{ controlGroup: ControlGroup }>()

const { getControl, updateGroup, deleteGroup, setGroupControls, deleteControl, handleCommand } =
  useControls()
const { openModal: ausOpenModal } = useAreYouSureModal()
const { openModal } = useControlModal()

const open = ref(localStorage.getItem(`controlGroupOpen_${props.controlGroup.id}`) !== 'false')
const editMode = ref(false)
const groupNameInput = ref<string>('')

const { getClients } = useClientsDb()

const commandsObservable = from(
  liveQuery<Command[]>(() => {
    const promises = props.controlGroup.controls.map((control) =>
      db.commands
        .where('[controlId+createdAt]')
        .between([control.id, 0], [control.id, Infinity])
        .reverse()
        .limit(1)
        .toArray()
        .then((commands) => commands[0])
    )
    return Promise.all(promises).then((commands) => commands.filter(Boolean))
  })
)

const clientsObservable = from(getClients())

const commands = useObservable(commandsObservable, { initialValue: [] as Command[] })
const clients = useObservable(clientsObservable, { initialValue: [] as Client[] })

const commandsLastUser = computed(() => {
  const record: Record<string, Client | undefined> = {}

  commands.value.forEach(
    (command) =>
      (record[command.controlId] = clients.value.find(
        (client) => client.ip === command.ip && client.discordId === (command.discordId || null)
      ))
  )

  return record
})

const openModalValue = computed({
  get: () => open.value || editMode.value,
  set: (value: boolean) => (open.value = value)
})

const items = computed<DropdownMenuItem[]>(() => [
  {
    label: 'Add Control',
    color: 'primary',
    icon: 'i-lucide-plus-circle',
    onSelect: () => openModal(props.controlGroup.id)
  },
  { type: 'separator' },
  {
    label: 'Delete',
    icon: 'i-lucide-trash',
    color: 'error',
    onSelect: async () => {
      const result = await ausOpenModal({
        title: 'Delete Control Group',
        message: `Are you sure you want to delete the control group "${props.controlGroup.name}"?\n\nThis action cannot be undone.`,
        confirmText: 'Delete'
      })

      if (result) deleteGroup(props.controlGroup.id)
    }
  }
])

const controls = computed({
  get: () => props.controlGroup.controls,
  set: (value) => {
    setGroupControls(props.controlGroup.id, value)
  }
})

const save = (): void => {
  updateGroup(props.controlGroup.id, groupNameInput.value)
  editMode.value = false
}

const edit = (): void => {
  editMode.value = true
  groupNameInput.value = props.controlGroup.name
}

const handleDeleteControl = async (controlId: string): Promise<void> => {
  const result = await ausOpenModal({
    title: 'Delete Control',
    message: `Are you sure you want to delete this control?\n\nThis action cannot be undone.`,
    confirmText: 'Delete'
  })

  if (result) {
    deleteControl(props.controlGroup.id, controlId)
  }
}

const handleCommandEvent = (controlId: string, command: CommandWithoutIds): void => {
  const control = getControl(props.controlGroup.id, controlId)

  if (control) handleCommand(control, command)
}

watch(open, (value) => {
  localStorage.setItem(`controlGroupOpen_${props.controlGroup.id}`, String(value))
})
</script>

<template>
  <UCard :ui="{ body: 'p-4 sm:p-4 space-y-4' }">
    <div class="flex gap-2">
      <UButton
        v-if="!editMode"
        :label="controlGroup.name"
        color="neutral"
        variant="subtle"
        trailing-icon="i-lucide-chevron-down"
        block
        :ui="{
          trailingIcon: ['transition-transform', open ? 'duration-200 rotate-180' : '']
        }"
        @click="open = !open"
      />

      <UInput
        v-else
        v-model="groupNameInput"
        placeholder="Control Group Name"
        class="grow"
      />

      <UButton
        v-if="editMode"
        color="success"
        variant="subtle"
        label="Save"
        icon="i-lucide-check"
        @click="save"
      />

      <UTooltip v-else>
        <UButton
          color="neutral"
          variant="subtle"
          icon="lucide:pen-line"
          @click="edit"
        />

        <template #content>
          Edit Control Group
        </template>
      </UTooltip>

      <!-- z-20: this control group can scroll up under AppHeader.vue's sticky z-10 bar - without
      its own explicit z-index above that, this (like every Nuxt UI overlay, none of which set one
      by default) would render behind it instead of floating over it. -->
      <UDropdownMenu
        :items="items"
        :content="{ align: 'end', side: 'bottom', sideOffset: 8 }"
        :ui="{ content: 'w-48 z-20' }"
      >
        <UButton
          icon="i-lucide-menu"
          color="neutral"
          variant="outline"
        />
      </UDropdownMenu>
    </div>

    <UCollapsible
      v-model:open="openModalValue"
      class="flex w-[90vw] flex-col gap-2 sm:w-114 md:w-173"
      :ui="{ content: 'overflow-visible' }"
    >
      <template #content>
        <!-- forceFallback: Sortable's default drag uses the native HTML5 Drag and Drop API, which
        shows the browser's own "not a valid drop target" (no-entry) cursor the moment its
        dragover/drop handlers don't fire the way it expects to somewhere in this DOM - which is
        what was actually happening, not a broken handle selector. Forcing its JS/CSS-based
        fallback (a floating clone that tracks the pointer) sidesteps native DnD entirely.

        group: every ControlGroup instance renders its own separate VueDraggable, each bound only
        to its own controlGroup.controls - giving them all the same Sortable group name is what
        lets a drag started in one accept a drop into another. vue-draggable-plus handles syncing
        both sides' v-model (removing from the source group, inserting into the destination one)
        on its own; nothing else here needs to change for that. The destination group doesn't need
        to be in edit mode itself to receive a drop - handle only gates starting a drag, not
        accepting one - it just needs to be expanded (open) to have a visible drop target at all. -->
        <VueDraggable
          v-model.lazy="controls"
          group="control-groups"
          handle=".handle"
          easing="cubic-bezier(0.25, 0.8, 0.25, 1)"
          :animation="200"
          :force-fallback="true"
          class="grid grid-cols-[repeat(2,minmax(0,180px))] items-center justify-center-safe gap-4 sm:grid-cols-[repeat(2,minmax(0,220px))] md:grid-cols-[repeat(3,minmax(0,220px))]"
        >
          <Control
            v-for="control in controls"
            :key="control.id"
            :control="control"
            :edit="editMode"
            :last-user="commandsLastUser[control.id]"
            :locked-indicator="control.locked"
            :unavailable-indicator="control.unavailable"
            @edit="openModal(controlGroup.id, control.id)"
            @delete="handleDeleteControl(control.id)"
            @command="handleCommandEvent(control.id, $event)"
          >
            <template #last-user>
              <UCard :ui="{ body: 'p-2 sm:p-2 space-y-2' }">
                <CommandDbTest :control-id="control.id" />
              </UCard>
            </template>
          </Control>
        </VueDraggable>
      </template>
    </UCollapsible>
  </UCard>
</template>

<style scoped></style>
