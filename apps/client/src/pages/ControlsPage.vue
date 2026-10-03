<script setup lang="ts">
import type { DropdownMenuItem } from '@nuxt/ui'
import type { ContextMenuItem } from '@nuxt/ui/components/ContextMenu.vue'
import CommandLine from '@renderer/components/controls/CommandLine.vue'
import GroupListItem from '@renderer/components/controls/GroupListItem.vue'
import EmptyState from '@renderer/components/ui/EmptyState.vue'
import SegmentedControl from '@renderer/components/ui/SegmentedControl.vue'
import { useAreYouSureModal } from '@renderer/composables/useAreYouSureModal'
import { useAvatarDetails } from '@renderer/composables/useAvatarDetails'
import { useControlActivity } from '@renderer/composables/useControlActivity'
import { useControls } from '@renderer/composables/useControls'
import type { TileDensity } from '@renderer/composables/useControlsView'
import { useControlsView } from '@renderer/composables/useControlsView'
import { useLockedControls } from '@renderer/composables/useLockedControls'
import { useOscConnection } from '@renderer/composables/useOscConnection'
import { usePresets } from '@renderer/composables/usePresets'
import type { CommandWithoutIds, ControlGroup, ControlType, PresetControl } from '@toolkitosc/shared-ui'
import { Control, controlTileSpan } from '@toolkitosc/shared-ui'
import { computed, nextTick, ref, watch } from 'vue'
import { VueDraggable } from 'vue-draggable-plus'
import { useRoute, useRouter } from 'vue-router'

// The Controls page: groups in the sidebar, the selected group's tiles filling the page. One group
// at a time means tiles can be large (easy to hit from VR) no matter how many controls an avatar
// has. Use mode is for using the controls; Edit mode shows per-tile Edit/Activity/Delete buttons,
// drag handles, and group checkboxes for bulk actions - so nothing can be moved or deleted by
// accident while in use.
const PINNED = 'pinned'
// Every preset of the current avatar as a tile, so applying one is a single tap - without having
// to make a Preset control for each of them first.
const PRESETS = 'presets'

const route = useRoute()
const router = useRouter()
const {
  controls,
  setGroups,
  addGroup,
  updateGroup,
  deleteGroup,
  deleteGroups,
  duplicateGroup,
  setGroupHidden,
  setGroupsHidden,
  setGroupControls,
  deleteControl,
  moveControl,
  setControlPinned,
  getControl,
  handleCommand
} = useControls()
const { connected: oscConnected } = useOscConnection()
const { avatarDetails } = useAvatarDetails()
const { lockedControlGroups, currentLockedControlsGroup, defaultLockedControlsGroup } = useLockedControls()
const { density, editMode } = useControlsView()
const { openModal: confirm } = useAreYouSureModal()
const { presets, applyPreset } = usePresets()
const toast = useToast()

// Groups are keyed by avatar id (see useControls.ts), so there's nothing to show or add to until
// VRChat's OSC is alive and has said which avatar is loaded.
const ready = computed(() => oscConnected.value && !!avatarDetails.value)

const query = ref('')
const search = computed(() => query.value.trim().toLowerCase())

const selectedId = computed(() => {
  const param = route.params.groupId
  const id = Array.isArray(param) ? param[0] : param

  if (id === PINNED || id === PRESETS) return id

  return controls.value.find((group) => group.id === id)?.id ?? controls.value[0]?.id ?? null
})

const selectedGroup = computed(() => controls.value.find((group) => group.id === selectedId.value) ?? null)

const selectGroup = (id: string): void => {
  query.value = ''
  void router.replace(`/controls/${id}`)
}

// What the tile area shows: search results across every group, the pinned controls, or the
// selected group. Each tile keeps its own group id, since commands and edits are per group.
interface ShownControl {
  control: ControlType
  groupId: string
}

const shown = computed<ShownControl[]>(() => {
  if (search.value) {
    return controls.value.flatMap((group) =>
      group.controls.filter((control) => control.name.toLowerCase().includes(search.value)).map((control) => ({ control, groupId: group.id }))
    )
  }

  if (selectedId.value === PINNED) {
    return controls.value.flatMap((group) => group.controls.filter((control) => control.pinned).map((control) => ({ control, groupId: group.id })))
  }

  return selectedGroup.value?.controls.map((control) => ({ control, groupId: selectedGroup.value!.id })) ?? []
})

// The Presets view's tiles: stand-in Preset controls, not stored anywhere. Applying one goes
// straight to usePresets, and right-clicking one offers the preset's own actions.
const presetTiles = computed<PresetControl[]>(() =>
  presets.value.map((preset) => ({ id: `preset:${preset.id}`, type: 'preset', name: preset.name, icon: preset.icon || undefined, presetId: preset.id }))
)

const runPresetTile = (tile: PresetControl): void => applyPreset(tile.presetId)

const menuPreset = ref<PresetControl | null>(null)

const pickPreset = (event: MouseEvent): void => {
  const id = closestData(event, 'controlId')

  menuPreset.value = presetTiles.value.find((tile) => tile.id === id) ?? null
}

const presetContextItems = (tile: PresetControl): ContextMenuItem[][] => [
  [
    { label: 'Apply', icon: 'i-lucide-play', onSelect: () => runPresetTile(tile) },
    { label: 'Edit preset', icon: 'i-lucide-pen', onSelect: () => void router.push(`/presets/${tile.presetId}`) }
  ]
]

const pinnedCount = computed(() => controls.value.reduce((n, group) => n + group.controls.filter((c) => c.pinned).length, 0))

const builtInViews = computed(() => [
  { id: PINNED, label: 'Pinned', icon: 'i-lucide-star', count: pinnedCount.value, hint: `${pinnedCount.value} pinned from any group` },
  { id: PRESETS, label: 'Presets', icon: 'i-lucide-layers', count: presets.value.length, hint: `${presets.value.length} presets for this avatar` }
])

const sidebarGroups = computed(() =>
  search.value
    ? controls.value
        .map((group) => ({ group, matches: group.controls.filter((c) => c.name.toLowerCase().includes(search.value)).length }))
        .filter(({ group, matches }) => matches > 0 || group.name.toLowerCase().includes(search.value))
    : controls.value.map((group) => ({ group, matches: undefined }))
)

// Drag-to-reorder models. Groups only reorder in Edit mode and outside a search (a filtered list
// can't be reordered meaningfully); tiles only within a real group.
const groupsModel = computed<ControlGroup[]>({
  get: () => controls.value,
  set: (value) => setGroups(value)
})

const tilesModel = computed<ControlType[]>({
  get: () => selectedGroup.value?.controls ?? [],
  set: (value) => selectedGroup.value && setGroupControls(selectedGroup.value.id, value)
})

const canReorderTiles = computed(() => editMode.value && !search.value && !!selectedGroup.value && selectedId.value !== PINNED)

// Bulk selection (Edit mode only).
const selectedGroupIds = ref<Set<string>>(new Set())

watch(editMode, () => (selectedGroupIds.value = new Set()))

const toggleGroupSelection = (id: string): void => {
  const next = new Set(selectedGroupIds.value)

  if (next.has(id)) next.delete(id)
  else next.add(id)

  selectedGroupIds.value = next
}

const allSelected = computed(() => controls.value.length > 0 && selectedGroupIds.value.size === controls.value.length)

const toggleSelectAll = (): void => {
  selectedGroupIds.value = allSelected.value ? new Set() : new Set(controls.value.map((group) => group.id))
}

const bulkHide = (): void => {
  const ids = [...selectedGroupIds.value]
  // Hide all, unless every selected group is already hidden - then show them.
  const hide = !ids.every((id) => controls.value.find((group) => group.id === id)?.hidden)

  setGroupsHidden(ids, hide)
  selectedGroupIds.value = new Set()
  toast.add({ title: hide ? `Hid ${ids.length} groups from viewers` : `Showing ${ids.length} groups to viewers`, icon: hide ? 'i-lucide-eye-off' : 'i-lucide-eye' })
}

const bulkDelete = async (): Promise<void> => {
  const count = selectedGroupIds.value.size
  const ok = await confirm({
    title: `Delete ${count} ${count === 1 ? 'group' : 'groups'}?`,
    message: `This deletes **${count} ${count === 1 ? 'group' : 'groups'}** and every control in them. This can't be undone.`,
    confirmText: 'Delete'
  })

  if (!ok) return

  deleteGroups([...selectedGroupIds.value])
  selectedGroupIds.value = new Set()
  toast.add({ title: `Deleted ${count} ${count === 1 ? 'group' : 'groups'}`, icon: 'i-lucide-trash-2' })
}

// The control editor is a page; controlId omitted means a new control in that group.
const openEditor = (groupId: string, controlId?: string): void => {
  void router.push(controlId ? `/controls/${groupId}/${controlId}` : `/controls/${groupId}/new`)
}

const openActivity = (controlId: string): void => {
  void router.push({ path: '/activity', query: { control: controlId, range: 'all' } })
}

const newGroup = (): void => {
  const id = addGroup()

  selectGroup(id)
}

// Renaming happens in place of the page title.
const renaming = ref(false)
const renameValue = ref('')

// Opens the group first when started from another group's menu - selecting a group cancels a
// rename (see the watch below), so the rename starts once the switch has happened.
const startRename = async (groupId = selectedGroup.value?.id): Promise<void> => {
  if (!groupId) return

  if (groupId !== selectedId.value) {
    query.value = ''
    await router.replace(`/controls/${groupId}`)
    await nextTick()
  }

  renameValue.value = controls.value.find((group) => group.id === groupId)?.name ?? ''
  renaming.value = true
}

const saveRename = (): void => {
  const name = renameValue.value.trim()

  if (selectedGroup.value && name) updateGroup(selectedGroup.value.id, name)
  renaming.value = false
}

watch(selectedId, () => (renaming.value = false))

const removeGroup = async (group: ControlGroup | null = selectedGroup.value): Promise<void> => {
  if (!group) return

  const ok = await confirm({
    title: 'Delete group?',
    message: `Delete **${group.name}** and its ${group.controls.length} controls? This can't be undone.`,
    confirmText: 'Delete'
  })

  if (ok) deleteGroup(group.id)
}

const moveGroup = (groupId: string, by: -1 | 1): void => {
  const list = [...controls.value]
  const from = list.findIndex((group) => group.id === groupId)
  const to = from + by

  if (from < 0 || to < 0 || to >= list.length) return

  list.splice(to, 0, ...list.splice(from, 1))
  setGroups(list)
}

const copyGroup = (group: ControlGroup): void => {
  const id = duplicateGroup(group.id)

  if (!id) return

  selectGroup(id)
  toast.add({ title: `Duplicated ${group.name}`, icon: 'i-lucide-copy' })
}

const toggleGroupHidden = (group: ControlGroup): void => {
  setGroupHidden(group.id, !group.hidden)
  toast.add({ title: group.hidden ? `Showing ${group.name} to viewers` : `Hid ${group.name} from viewers`, icon: group.hidden ? 'i-lucide-eye' : 'i-lucide-eye-off' })
}

// A group's actions, from its row in the sidebar: right-click it, or press "⋯" on the selected one.
// Works in Use mode too - only bulk selection and drag-to-reorder need Edit mode. Move up/down is
// the pointer-friendly way to reorder (dragging a small handle is hard from VR).
const groupMenu = (group: ControlGroup): DropdownMenuItem[][] => {
  const index = controls.value.findIndex((g) => g.id === group.id)

  return [
    [
      { label: 'Add control', icon: 'i-lucide-plus', onSelect: () => openEditor(group.id) },
      { label: 'Rename', icon: 'i-lucide-pen', onSelect: () => void startRename(group.id) },
      { label: 'Duplicate', icon: 'i-lucide-copy', onSelect: () => copyGroup(group) },
      {
        label: group.hidden ? 'Show to viewers' : 'Hide from viewers',
        icon: group.hidden ? 'i-lucide-eye' : 'i-lucide-eye-off',
        onSelect: () => toggleGroupHidden(group)
      }
    ],
    [
      { label: 'Move up', icon: 'i-lucide-arrow-up', disabled: index <= 0, onSelect: () => moveGroup(group.id, -1) },
      { label: 'Move down', icon: 'i-lucide-arrow-down', disabled: index >= controls.value.length - 1, onSelect: () => moveGroup(group.id, 1) }
    ],
    [{ label: 'Delete', icon: 'i-lucide-trash-2', color: 'error', onSelect: () => void removeGroup(group) }]
  ]
}

const deleteTile = async ({ control, groupId }: ShownControl): Promise<void> => {
  const ok = await confirm({
    title: 'Delete control?',
    message: `Delete **${control.name}**? This can't be undone.`,
    confirmText: 'Delete'
  })

  if (ok) deleteControl(groupId, control.id)
}

// Moves a tile to another group, with Undo in the toast that puts it back where it was.
const moveTile = ({ control, groupId }: ShownControl, toGroupId: string): void => {
  const target = controls.value.find((group) => group.id === toGroupId)
  const fromIndex = moveControl(groupId, control.id, toGroupId)

  if (fromIndex === undefined || !target) return

  toast.add({
    title: `Moved ${control.name} to ${target.name}`,
    icon: 'i-lucide-folder-input',
    actions: [{ label: 'Undo', color: 'neutral', variant: 'outline', onClick: () => void moveControl(toGroupId, control.id, groupId, fromIndex) }]
  })
}

// The "Move to" submenu: every other group, in sidebar order.
const moveItems = (item: ShownControl): ContextMenuItem[] =>
  controls.value
    .filter((group) => group.id !== item.groupId)
    .map((group) => ({ label: group.name, icon: group.hidden ? 'i-lucide-eye-off' : 'i-lucide-folder', onSelect: () => moveTile(item, group.id) }))

const togglePinned = ({ control, groupId }: ShownControl): void => {
  setControlPinned(groupId, control.id, !control.pinned)
  toast.add({ title: control.pinned ? `Unpinned ${control.name}` : `Pinned ${control.name}`, icon: 'i-lucide-pin' })
}

// Right-click shortcuts for desktop users; the same actions are on the tiles in Edit mode for VR.
//
// One context menu per list, not one per tile or group row: wrapping each draggable item in its own
// UContextMenu breaks drag-to-reorder (Sortable drops the item at the wrong place and the order
// never changes). So each list has a single menu, and a capture-phase contextmenu listener records
// which tile or row was right-clicked just before it opens - Reka waits a tick before opening, so
// the items and `disabled` are already up to date by then. Right-clicking empty space opens nothing.
const menuTile = ref<ShownControl | null>(null)
const menuGroup = ref<ControlGroup | null>(null)

const closestData = (event: MouseEvent, key: 'controlId' | 'groupId'): string | undefined =>
  (event.target as HTMLElement).closest<HTMLElement>(`[data-${key === 'controlId' ? 'control-id' : 'group-id'}]`)?.dataset[key]

const pickTile = (event: MouseEvent): void => {
  const id = closestData(event, 'controlId')

  menuTile.value = shown.value.find((item) => item.control.id === id) ?? null
}

const pickGroup = (event: MouseEvent): void => {
  const id = closestData(event, 'groupId')

  menuGroup.value = controls.value.find((group) => group.id === id) ?? null
}

const contextItems = (item: ShownControl): ContextMenuItem[][] => [
  [
    { label: 'Edit', icon: 'i-lucide-pen', onSelect: () => openEditor(item.groupId, item.control.id) },
    { label: 'Activity', icon: 'i-lucide-history', onSelect: () => openActivity(item.control.id) },
    { label: item.control.pinned ? 'Unpin' : 'Pin', icon: 'i-lucide-pin', onSelect: () => togglePinned(item) },
    { label: 'Move to', icon: 'i-lucide-folder-input', disabled: controls.value.length < 2, children: moveItems(item) }
  ],
  [{ label: 'Delete', icon: 'i-lucide-trash-2', color: 'error', onSelect: () => deleteTile(item) }]
]

const onCommand = ({ control, groupId }: ShownControl, command: CommandWithoutIds): void => {
  const fresh = getControl(groupId, control.id)

  if (fresh) handleCommand(fresh, command)
}

const { lastUsers, latest } = useControlActivity(computed(() => shown.value.map(({ control }) => control.id)))

const activeProfile = computed(() => lockedControlGroups.value.find((group) => group.id === currentLockedControlsGroup.value) ?? null)

const title = computed(() => {
  if (search.value) return 'Search'
  if (selectedId.value === PINNED) return 'Pinned'
  if (selectedId.value === PRESETS) return 'Presets'

  return selectedGroup.value?.name ?? 'Controls'
})

const meta = computed(() => {
  if (search.value) return `${shown.value.length} controls match "${query.value.trim()}"`

  if (selectedId.value === PRESETS) return `${presets.value.length} ${presets.value.length === 1 ? 'preset' : 'presets'} for this avatar`

  const count = `${shown.value.length} ${shown.value.length === 1 ? 'control' : 'controls'}`

  return selectedGroup.value?.hidden && selectedId.value !== PINNED ? `${count} · hidden from viewers` : count
})

const densityItems: { value: TileDensity; label: string; title: string }[] = [
  { value: 's', label: 'S', title: 'Small tiles' },
  { value: 'm', label: 'M', title: 'Medium tiles' },
  { value: 'l', label: 'L', title: 'Large tiles' }
]

const mode = computed<'use' | 'edit'>({
  get: () => (editMode.value ? 'edit' : 'use'),
  set: (value) => (editMode.value = value === 'edit')
})
</script>

<template>
  <PageLayout>
    <template #sidebar>
      <div class="flex min-h-9 shrink-0 items-center gap-2">
        <b class="min-w-0 flex-1 text-xl font-semibold text-highlighted">Controls</b>
        <UButton
          v-if="editMode && ready"
          icon="i-lucide-plus"
          size="md"
          color="neutral"
          variant="subtle"
          @click="newGroup"
        >
          Group
        </UButton>
      </div>

      <UInput
        v-model="query"
        icon="i-lucide-search"
        placeholder="Search controls"
        class="shrink-0"
        :disabled="!ready"
        :ui="{ base: 'rounded-2xl' }"
      >
        <template
          v-if="query"
          #trailing
        >
          <UButton
            icon="i-lucide-x"
            color="neutral"
            variant="link"
            size="sm"
            aria-label="Clear search"
            @click="query = ''"
          />
        </template>
      </UInput>

      <nav
        v-if="ready"
        aria-label="Groups"
        class="-mr-1.5 flex min-h-0 flex-1 flex-col gap-1.5 overflow-y-auto pr-1.5"
      >
        <template v-if="!search">
          <!-- The two built-in views, side by side as big icon buttons so they take one row
          above the groups instead of two. -->
          <div class="grid shrink-0 grid-cols-2 gap-1.5">
            <UTooltip
              v-for="view in builtInViews"
              :key="view.id"
              :text="view.hint"
            >
              <button
                type="button"
                class="flex h-15 cursor-pointer flex-col items-center justify-center gap-1 rounded-2xl border transition-colors"
                :class="selectedId === view.id ? 'border-primary/50 bg-primary/14 text-highlighted' : 'border-transparent bg-(--aurora-well) text-muted hover:bg-(--aurora-glass)'"
                :aria-pressed="selectedId === view.id"
                @click="selectGroup(view.id)"
              >
                <UIcon
                  :name="view.icon"
                  class="size-5"
                  :class="selectedId === view.id ? 'text-primary' : ''"
                />
                <span class="flex items-center gap-1.5 text-xs font-semibold">
                  {{ view.label }}
                  <span class="font-mono text-[10.5px] font-normal text-muted">{{ view.count }}</span>
                </span>
              </button>
            </UTooltip>
          </div>
          <div class="mx-1 h-px shrink-0 bg-(--ui-border)" />
        </template>

        <UContextMenu
          :items="menuGroup ? groupMenu(menuGroup) : []"
          :disabled="!menuGroup"
        >
          <div @contextmenu.capture="pickGroup">
            <VueDraggable
              v-model="groupsModel"
              handle=".group-handle"
              :disabled="!editMode || !!search"
              :animation="200"
              :force-fallback="true"
              class="flex flex-col gap-1.5"
            >
              <GroupListItem
                v-for="{ group, matches } in sidebarGroups"
                :key="group.id"
                :group="group"
                :matches="matches"
                :active="!search && group.id === selectedId"
                :editing="editMode"
                :selected="selectedGroupIds.has(group.id)"
                :menu="groupMenu(group)"
                @select="selectGroup(group.id)"
                @toggle-select="toggleGroupSelection(group.id)"
              />
            </VueDraggable>
          </div>
        </UContextMenu>

        <p
          v-if="search && !sidebarGroups.length"
          class="px-2.5 text-sm text-muted"
        >
          No groups match.
        </p>
      </nav>
      <p
        v-else
        class="text-sm text-muted"
      >
        Groups show up here once VRChat sends your avatar.
      </p>

      <div
        v-if="editMode && ready"
        class="grid shrink-0 gap-2"
      >
        <div class="flex items-center justify-between px-1 font-mono text-[10.5px] tracking-widest text-muted uppercase">
          <span>{{ selectedGroupIds.size }} selected</span>
          <button
            type="button"
            class="cursor-pointer text-primary"
            @click="toggleSelectAll"
          >
            {{ allSelected ? 'Select none' : 'Select all' }}
          </button>
        </div>
        <div class="grid grid-cols-2 gap-1.5">
          <UButton
            icon="i-lucide-eye-off"
            color="neutral"
            variant="subtle"
            size="md"
            block
            :disabled="!selectedGroupIds.size"
            @click="bulkHide"
          >
            Hide
          </UButton>
          <UButton
            icon="i-lucide-trash-2"
            color="error"
            variant="soft"
            size="md"
            block
            :disabled="!selectedGroupIds.size"
            @click="bulkDelete"
          >
            Delete
          </UButton>
        </div>
      </div>
    </template>

    <template #header>
      <template v-if="renaming">
        <form
          class="flex min-w-0 flex-1 items-center gap-2"
          @submit.prevent="saveRename"
        >
          <UInput
            v-model="renameValue"
            autofocus
            size="xl"
            class="min-w-0 flex-1"
            aria-label="Group name"
            @keydown.esc="renaming = false"
          />
          <UButton
            type="submit"
            icon="i-lucide-check"
          >
            Save
          </UButton>
          <UButton
            color="neutral"
            variant="ghost"
            @click="renaming = false"
          >
            Cancel
          </UButton>
        </form>
      </template>
      <PageHeader
        v-else
        :title="ready ? title : 'Controls'"
        :meta="ready ? meta : undefined"
      >
        <template
          v-if="ready"
          #actions
        >
          <!-- The open group's actions (Edit mode) and the Presets link sit left of the view
          toggles, in the same glass pill and button size as them, so the toggles keep a fixed
          place at the end of the header whatever the mode. -->
          <div
            v-if="editMode && selectedGroup && selectedId !== PINNED && !search"
            class="flex shrink-0 gap-0.5 rounded-[calc(var(--ui-radius)*4)] glass p-1"
            role="group"
            aria-label="Group actions"
          >
            <button
              type="button"
              class="flex h-10 cursor-pointer items-center gap-1.5 rounded-2xl px-3.5 text-sm font-medium text-default transition-colors hover:bg-accented"
              @click="startRename()"
            >
              <UIcon
                name="i-lucide-pen"
                class="size-4"
              />
              Rename
            </button>
            <button
              type="button"
              class="flex h-10 cursor-pointer items-center gap-1.5 rounded-2xl px-3.5 text-sm font-medium text-default transition-colors hover:bg-accented"
              @click="toggleGroupHidden(selectedGroup)"
            >
              <UIcon
                :name="selectedGroup.hidden ? 'i-lucide-eye' : 'i-lucide-eye-off'"
                class="size-4"
              />
              {{ selectedGroup.hidden ? 'Show' : 'Hide' }}
            </button>
            <button
              type="button"
              class="grid h-10 min-w-11 cursor-pointer place-items-center rounded-2xl text-error transition-colors hover:bg-error/15"
              title="Delete group"
              aria-label="Delete group"
              @click="removeGroup()"
            >
              <UIcon
                name="i-lucide-trash-2"
                class="size-4"
              />
            </button>
            <button
              type="button"
              class="flex h-10 cursor-pointer items-center gap-1.5 rounded-2xl bg-primary px-3.5 text-sm font-semibold text-inverted transition-colors hover:bg-primary/85"
              @click="openEditor(selectedGroup.id)"
            >
              <UIcon
                name="i-lucide-plus"
                class="size-4"
              />
              Add control
            </button>
          </div>
          <div
            v-if="selectedId === PRESETS && !search"
            class="flex shrink-0 rounded-[calc(var(--ui-radius)*4)] glass p-1"
          >
            <RouterLink
              to="/presets"
              class="flex h-10 items-center gap-1.5 rounded-2xl px-3.5 text-sm font-medium text-default transition-colors hover:bg-accented"
            >
              <UIcon
                name="i-lucide-layers"
                class="size-4"
              />
              Manage presets
            </RouterLink>
          </div>
          <SegmentedControl
            v-model="density"
            :items="densityItems"
            aria-label="Tile size"
          />
          <SegmentedControl
            v-model="mode"
            accent
            aria-label="Mode"
            :items="[
              { value: 'use', label: 'Use', icon: 'i-lucide-play' },
              { value: 'edit', label: 'Edit', icon: 'i-lucide-pen' }
            ]"
          />
        </template>
      </PageHeader>
    </template>

    <template v-if="ready">
      <div
        v-if="activeProfile"
        class="mb-3 flex items-center gap-3 rounded-field border border-warning/40 bg-warning/10 px-3.5 py-3 text-sm"
      >
        <UIcon
          name="i-lucide-lock"
          class="size-5 shrink-0 text-warning"
        />
        <div class="min-w-0 flex-1">
          Profile "{{ activeProfile.name }}" is on{{ activeProfile.id === defaultLockedControlsGroup ? ' (default for this avatar)' : '' }}
          <small class="block text-muted">Controls it locks or limits show a badge; viewers are held to it until you turn it off.</small>
        </div>
        <UButton
          size="md"
          color="neutral"
          variant="subtle"
          @click="currentLockedControlsGroup = null"
        >
          Turn off
        </UButton>
      </div>

      <template v-if="selectedId === PRESETS && !search">
        <UContextMenu
          v-if="presetTiles.length"
          :items="menuPreset ? presetContextItems(menuPreset) : []"
          :disabled="!menuPreset"
        >
          <div
            class="tile-grid"
            :data-density="density"
            @contextmenu.capture="pickPreset"
          >
            <div
              v-for="tile in presetTiles"
              :key="tile.id"
              data-span="s"
              :data-control-id="tile.id"
            >
              <Control
                :control="tile"
                @command="runPresetTile(tile)"
              />
            </div>
          </div>
        </UContextMenu>
        <EmptyState
          v-else
          icon="i-lucide-layers"
          title="No presets yet"
          description="A preset saves the current value of your avatar's parameters, so you can bring a whole look back in one tap. Its tile shows up here."
        >
          <template #actions>
            <UButton
              icon="i-lucide-camera"
              to="/presets/new"
            >
              New from current state
            </UButton>
          </template>
        </EmptyState>
      </template>

      <UContextMenu
        v-else-if="shown.length"
        :items="menuTile ? contextItems(menuTile) : []"
        :disabled="!menuTile"
      >
        <div @contextmenu.capture="pickTile">
          <VueDraggable
            v-model="tilesModel"
            handle=".handle"
            :disabled="!canReorderTiles"
            easing="cubic-bezier(0.25, 0.8, 0.25, 1)"
            :animation="200"
            :force-fallback="true"
            class="tile-grid"
            :data-density="density"
          >
            <!-- A wrapper div as the grid item, carrying data-span for wide and large tiles. -->
            <div
              v-for="item in shown"
              :key="item.control.id"
              :data-span="controlTileSpan(item.control)"
              :data-control-id="item.control.id"
            >
              <Control
                :control="item.control"
                :edit="editMode"
                :last-user="lastUsers[item.control.id]"
                :locked-indicator="item.control.locked"
                :unavailable-indicator="item.control.unavailable"
                :limited-indicator="!!item.control.limits"
                @edit="openEditor(item.groupId, item.control.id)"
                @logs="openActivity(item.control.id)"
                @delete="deleteTile(item)"
                @command="onCommand(item, $event)"
              />
            </div>
          </VueDraggable>
        </div>
      </UContextMenu>

      <EmptyState
        v-else-if="search"
        icon="i-lucide-search"
        :title="`Nothing matches &quot;${query.trim()}&quot;`"
        description="Try another word, or clear the search to go back to your groups."
      />
      <EmptyState
        v-else-if="selectedId === PINNED"
        icon="i-lucide-star"
        title="Nothing pinned yet"
        description="Pin the controls you use most to reach them here from any group. Right-click a tile, or open it in Edit mode."
      />
      <EmptyState
        v-else-if="selectedGroup"
        icon="i-lucide-layout-grid"
        title="No controls in this group"
        description="Add toggles, sliders and pickers for your avatar's parameters."
      >
        <template #actions>
          <UButton
            icon="i-lucide-plus"
            @click="openEditor(selectedGroup.id)"
          >
            Add control
          </UButton>
        </template>
      </EmptyState>
      <EmptyState
        v-else
        icon="i-lucide-folder-plus"
        title="No groups yet"
        description="Groups hold your controls, like Outfit, Face or Props."
      >
        <template #actions>
          <UButton
            icon="i-lucide-plus"
            @click="newGroup"
          >
            Create a group
          </UButton>
        </template>
      </EmptyState>
    </template>

    <EmptyState
      v-else
      icon="i-lucide-radio-tower"
      :title="oscConnected ? 'Waiting for an avatar' : 'VRChat not detected'"
      :description="
        oscConnected
          ? 'OSC is connected, but no avatar has been detected yet. Load into a world and it will show up here.'
          : 'No OSC data has been received yet. Make sure VRChat is running and OSC is enabled.'
      "
    >
      <!-- VRChat only sends avatar info once, right as OSC is enabled - if that happened before
      this app was listening, nothing will populate `avatarDetails` until OSC is toggled off and
      back on to resend it. -->
      <ol class="mt-2 grid list-decimal gap-1.5 pl-5 text-left leading-relaxed">
        <li v-if="!oscConnected">
          Launch VRChat and load into any world.
        </li>
        <li>
          Turn on OSC in <b class="font-medium text-default">Radial menu → Options → OSC → Enabled</b>
          or <b class="font-medium text-default">Settings → Avatars → OSC</b>.
        </li>
        <li>Already on? Turn it off and on again so VRChat resends your avatar.</li>
      </ol>
    </EmptyState>

    <template
      v-if="ready"
      #bar
    >
      <CommandLine :latest="latest" />
    </template>
  </PageLayout>
</template>
