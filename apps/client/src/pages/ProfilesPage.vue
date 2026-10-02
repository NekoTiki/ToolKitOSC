<script setup lang="ts">
import type { DropdownMenuItem } from '@nuxt/ui'
import EmptyState from '@renderer/components/ui/EmptyState.vue'
import FormSection from '@renderer/components/ui/FormSection.vue'
import { useAreYouSureModal } from '@renderer/composables/useAreYouSureModal'
import { useAvatarDetails } from '@renderer/composables/useAvatarDetails'
import { useControls } from '@renderer/composables/useControls'
import type { LockedControlGroup } from '@renderer/composables/useLockedControls'
import { useLockedControls } from '@renderer/composables/useLockedControls'
import type { ControlType } from '@toolkitosc/shared-ui'
import { CONTROL_TYPE_LABELS } from '@toolkitosc/shared-ui'
import _ from 'lodash'
import { computed, nextTick, onMounted, ref, useTemplateRef, watch } from 'vue'
import { onBeforeRouteLeave, useRoute, useRouter } from 'vue-router'

// Lock profiles: named sets of controls that are locked for everyone while the profile is on
// (a "stream safe" or "public world" mode, say). The list, with "No profile" first, lives in the
// sidebar; the selected profile opens in an editor with a switch per control and an Allow all
// switch per group. Replaces the two locked-control-group modals.
const route = useRoute()
const router = useRouter()
const toast = useToast()
const { avatarDetails } = useAvatarDetails()
const { controls } = useControls()
const { lockedControlGroups, currentLockedControlsGroup, getLockedControlGroup, addLockedControlGroup, updateLockedControlGroup, removeLockedControlGroup } =
  useLockedControls()
const { openModal: confirm } = useAreYouSureModal()

const isNew = route.name === 'profile-new'
const profileId = computed(() => (typeof route.params.profileId === 'string' ? route.params.profileId : null))
const profile = computed(() => (profileId.value ? getLockedControlGroup(profileId.value) : undefined))

// /profiles alone opens the first profile, if any.
watch(
  lockedControlGroups,
  (list) => {
    if (route.name === 'profiles' && list.length) void router.replace(`/profiles/${list[0]!.id}`)
  },
  { immediate: true }
)

interface Draft {
  name: string
  lockedControls: Record<string, boolean>
}

const initial = (): Draft | null => {
  if (isNew) return { name: '', lockedControls: {} }
  if (!profile.value) return null

  return { name: profile.value.name, lockedControls: _.cloneDeep(profile.value.lockedControls) }
}

const draft = ref<Draft | null>(initial())
let snapshot = JSON.stringify(draft.value)

watch(profile, (value) => {
  if (!draft.value && value) {
    draft.value = initial()
    snapshot = JSON.stringify(draft.value)
  }
})

const dirty = computed(() => !!draft.value && JSON.stringify(draft.value) !== snapshot)
const nameError = ref<string>()
const saved = ref(false)

const isLocked = (control: ControlType): boolean => !!draft.value?.lockedControls[control.id]

const setAllowed = (control: ControlType, allowed: boolean): void => {
  if (!draft.value) return

  if (allowed) delete draft.value.lockedControls[control.id]
  else draft.value.lockedControls[control.id] = true
}

const allowedCount = (controlsList: ControlType[]): number => controlsList.filter((control) => !isLocked(control)).length

const setGroupAllowed = (controlsList: ControlType[], allowed: boolean): void => controlsList.forEach((control) => setAllowed(control, allowed))

const lockedTotal = computed(() => Object.keys(draft.value?.lockedControls ?? {}).length)
const controlTotal = computed(() => controls.value.reduce((n, group) => n + group.controls.length, 0))

const isActive = computed(() => !!profileId.value && currentLockedControlsGroup.value === profileId.value)

// Shockers show their mode (shock or vibrate) rather than a generic icon.
const controlIcon = (control: ControlType): string =>
  control.type === 'open-shock-shocker' ? (control.mode === 'Shock' ? 'material-symbols:electric-bolt' : 'i-lucide-waves') : control.icon || 'i-lucide-square'

const save = (): boolean => {
  if (!draft.value) return false

  const name = draft.value.name.trim()

  if (!name) {
    nameError.value = 'Give the profile a name.'

    return false
  }

  const data = { name, lockedControls: draft.value.lockedControls }

  if (isNew) {
    saved.value = true
    const id = addLockedControlGroup(data)

    toast.add({ title: `Created "${name}"`, icon: 'i-lucide-check', color: 'success' })
    void router.push(`/profiles/${id}`)
  } else {
    updateLockedControlGroup(profileId.value!, data)
    snapshot = JSON.stringify(draft.value)
    toast.add({ title: `Saved "${name}"`, icon: 'i-lucide-check', color: 'success' })
  }

  return true
}

// Turning a profile on saves it first, so what gets enforced is what's on screen.
const turnOn = (): void => {
  if (dirty.value && !save()) return

  currentLockedControlsGroup.value = profileId.value
  toast.add({ title: `"${profile.value?.name}" is on`, icon: 'i-lucide-lock' })
}

const remove = async (): Promise<void> => {
  if (!profile.value) return

  const ok = await confirm({
    title: 'Delete profile?',
    message: `Delete **${profile.value.name}**?${isActive.value ? ' It is on right now, so its controls unlock.' : ''}`,
    confirmText: 'Delete'
  })

  if (!ok) return

  removeLockedControlGroup(profile.value.id)
  saved.value = true
  void router.push('/profiles')
}

// The name field, focused by the Rename menu item (directly, or after opening the profile with
// ?rename=1).
const nameInput = useTemplateRef<{ inputRef?: HTMLInputElement }>('nameInput')

const focusName = async (): Promise<void> => {
  await nextTick()
  nameInput.value?.inputRef?.focus()
  nameInput.value?.inputRef?.select()
}

onMounted(() => {
  if (route.query.rename) void focusName()
})

const duplicate = (item: LockedControlGroup): void => {
  const id = addLockedControlGroup({ name: `${item.name} copy`, lockedControls: _.cloneDeep(item.lockedControls) })

  toast.add({ title: `Duplicated "${item.name}"`, icon: 'i-lucide-copy' })
  void router.push(`/profiles/${id}`)
}

const removeFromMenu = async (item: LockedControlGroup): Promise<void> => {
  if (item.id === profileId.value) return remove()

  const ok = await confirm({
    title: 'Delete profile?',
    message: `Delete **${item.name}**?${currentLockedControlsGroup.value === item.id ? ' It is on right now, so its controls unlock.' : ''}`,
    confirmText: 'Delete'
  })

  if (ok) removeLockedControlGroup(item.id)
}

// A profile's actions, from its row in the sidebar: right-click it, or press "⋯" on the open one
// (VR pointers have no right-click). Turning on the open profile goes through turnOn(), which
// saves unsaved edits first.
const profileMenu = (item: LockedControlGroup): DropdownMenuItem[][] => {
  const on = currentLockedControlsGroup.value === item.id

  return [
    [
      on
        ? { label: 'Turn off', icon: 'i-lucide-lock-open', onSelect: () => (currentLockedControlsGroup.value = null) }
        : {
            label: 'Turn on',
            icon: 'i-lucide-lock',
            onSelect: () => {
              if (item.id === profileId.value) return turnOn()

              currentLockedControlsGroup.value = item.id
              toast.add({ title: `"${item.name}" is on`, icon: 'i-lucide-lock' })
            }
          },
      {
        label: 'Rename',
        icon: 'i-lucide-pen',
        onSelect: () => (item.id === profileId.value ? void focusName() : void router.push({ path: `/profiles/${item.id}`, query: { rename: '1' } }))
      },
      { label: 'Duplicate', icon: 'i-lucide-copy', onSelect: () => duplicate(item) }
    ],
    [{ label: 'Delete', icon: 'i-lucide-trash-2', color: 'error', onSelect: () => void removeFromMenu(item) }]
  ]
}

onBeforeRouteLeave(async () => {
  if (saved.value || !dirty.value) return true

  return confirm({ title: 'Discard changes?', message: 'Your changes to this profile will be lost.', confirmText: 'Discard' })
})
</script>

<template>
  <PageLayout>
    <template #sidebar>
      <div class="flex min-h-9 shrink-0 items-center">
        <b class="text-xl font-semibold text-highlighted">Profiles</b>
      </div>
      <p class="shrink-0 px-0.5 text-[13px] leading-snug text-muted">
        While a profile is on, the controls it locks can't be used by anyone.
      </p>

      <nav
        aria-label="Profiles"
        class="-mr-1.5 flex min-h-0 flex-1 flex-col gap-1.5 overflow-y-auto pr-1.5"
      >
        <button
          type="button"
          class="grid min-h-14.5 cursor-pointer grid-cols-[auto_minmax(0,1fr)_auto] items-center gap-x-2.5 rounded-2xl border border-transparent px-2.5 py-2 text-left transition-colors hover:bg-(--aurora-glass)"
          @click="currentLockedControlsGroup = null"
        >
          <span class="row-span-2 grid size-8.5 place-items-center rounded-xl bg-(--aurora-well) text-muted">
            <UIcon
              name="i-lucide-lock-open"
              class="size-4.5"
            />
          </span>
          <b class="truncate text-[14.5px] font-semibold text-highlighted">No profile</b>
          <span
            v-if="currentLockedControlsGroup === null"
            class="row-span-2 rounded-full bg-success/15 px-2 py-0.5 text-xs font-medium text-success"
          >Active</span>
          <small class="col-start-2 truncate text-xs text-muted">Everything unlocked</small>
        </button>
        <div class="mx-1 h-px shrink-0 bg-(--ui-border)" />

        <UContextMenu
          v-for="item in lockedControlGroups"
          :key="item.id"
          :items="profileMenu(item)"
        >
          <RouterLink
            :to="`/profiles/${item.id}`"
            class="grid min-h-14.5 grid-cols-[auto_minmax(0,1fr)_auto] items-center gap-x-2.5 rounded-2xl border px-2.5 py-2 transition-colors"
            :class="item.id === profileId ? 'border-primary/50 bg-primary/14' : 'border-transparent hover:bg-(--aurora-glass)'"
          >
            <span
              class="row-span-2 grid size-8.5 place-items-center rounded-xl bg-(--aurora-well)"
              :class="item.id === profileId ? 'text-primary' : 'text-muted'"
            >
              <UIcon
                name="i-lucide-lock"
                class="size-4.5"
              />
            </span>
            <b class="truncate text-[14.5px] font-semibold text-highlighted">{{ item.name }}</b>
            <span class="row-span-2 flex items-center gap-1">
              <span
                v-if="currentLockedControlsGroup === item.id"
                class="rounded-full bg-warning/15 px-2 py-0.5 text-xs font-medium text-warning"
              >On</span>
              <!-- .prevent: the menu button sits inside the link, and must not follow it. -->
              <UDropdownMenu
                v-if="item.id === profileId"
                :items="profileMenu(item)"
                :content="{ align: 'start', side: 'right' }"
              >
                <UButton
                  icon="i-lucide-ellipsis"
                  color="neutral"
                  variant="ghost"
                  size="md"
                  :aria-label="`${item.name} actions`"
                  @click.prevent.stop
                />
              </UDropdownMenu>
            </span>
            <small class="col-start-2 truncate text-xs text-muted">{{ Object.keys(item.lockedControls).length }} controls locked</small>
          </RouterLink>
        </UContextMenu>
      </nav>

      <UButton
        icon="i-lucide-plus"
        color="neutral"
        variant="subtle"
        block
        class="shrink-0"
        :disabled="!avatarDetails"
        to="/profiles/new"
      >
        New profile
      </UButton>
    </template>

    <template
      v-if="draft && avatarDetails"
      #header
    >
      <PageHeader
        :title="isNew ? 'New profile' : profile?.name ?? 'Profile'"
        :meta="`${lockedTotal} of ${controlTotal} controls locked`"
        :crumbs="[{ label: 'Profiles', to: '/profiles' }, { label: isNew ? 'New' : profile?.name ?? '' }]"
      >
        <template
          v-if="!isNew"
          #actions
        >
          <UButton
            icon="i-lucide-trash-2"
            color="error"
            variant="soft"
            @click="remove"
          >
            Delete
          </UButton>
          <UButton
            v-if="isActive"
            icon="i-lucide-lock-open"
            color="neutral"
            variant="subtle"
            @click="currentLockedControlsGroup = null"
          >
            Turn off
          </UButton>
          <UButton
            v-else
            icon="i-lucide-lock"
            @click="turnOn"
          >
            Turn on
          </UButton>
        </template>
      </PageHeader>
    </template>

    <EmptyState
      v-if="!avatarDetails"
      icon="i-lucide-radio-tower"
      title="VRChat not detected"
      description="Profiles belong to an avatar. Load into VRChat with OSC on to manage them."
    />

    <div
      v-else-if="draft"
      class="grid max-w-4xl grid-cols-1 gap-3.5"
    >
      <FormSection>
        <UFormField
          label="Name"
          required
          :error="nameError"
        >
          <UInput
            ref="nameInput"
            v-model="draft.name"
            placeholder="Stream safe, Public world…"
            class="w-full"
            @update:model-value="nameError = undefined"
          />
        </UFormField>
        <div
          v-if="isActive"
          class="flex items-center gap-3 rounded-field border border-warning/40 bg-warning/10 px-3.5 py-3 text-sm"
        >
          <UIcon
            name="i-lucide-lock"
            class="size-5 shrink-0 text-warning"
          />
          <span>This profile is on. Locked controls show a lock and can't be used by anyone, including you.</span>
        </div>
      </FormSection>

      <FormSection
        v-for="group in controls"
        :key="group.id"
        :title="group.name"
        :description="`${allowedCount(group.controls)} of ${group.controls.length} allowed`"
      >
        <template #actions>
          <USwitch
            label="Allow all"
            :model-value="allowedCount(group.controls) === group.controls.length"
            :disabled="!group.controls.length"
            @update:model-value="setGroupAllowed(group.controls, $event)"
          />
        </template>

        <div class="grid divide-y divide-(--ui-border)">
          <div
            v-for="control in group.controls"
            :key="control.id"
            class="flex min-h-14.5 items-center gap-3.5 py-2"
          >
            <span class="grid size-10 shrink-0 place-items-center rounded-md bg-(--aurora-well) text-muted">
              <UIcon
                :name="controlIcon(control)"
                class="size-5"
              />
            </span>
            <div class="grid min-w-0 flex-1">
              <b class="truncate font-medium text-highlighted">{{ control.name }}</b>
              <small class="text-xs text-muted">{{ CONTROL_TYPE_LABELS[control.type] }}</small>
            </div>
            <span
              class="rounded-full px-2.5 py-0.5 text-xs font-medium"
              :class="isLocked(control) ? 'bg-warning/15 text-warning' : 'bg-success/15 text-success'"
            >{{ isLocked(control) ? 'Locked' : 'Allowed' }}</span>
            <USwitch
              :model-value="!isLocked(control)"
              :aria-label="`Allow ${control.name}`"
              @update:model-value="setAllowed(control, $event)"
            />
          </div>
          <p
            v-if="!group.controls.length"
            class="py-2 text-sm text-muted"
          >
            No controls in this group.
          </p>
        </div>
      </FormSection>
    </div>

    <EmptyState
      v-else
      icon="i-lucide-lock"
      title="No profiles yet"
      description="A profile locks chosen controls for everyone at once, for example while streaming or in a public world."
    >
      <template #actions>
        <UButton
          icon="i-lucide-plus"
          to="/profiles/new"
        >
          New profile
        </UButton>
      </template>
    </EmptyState>

    <template
      v-if="draft && avatarDetails"
      #footer
    >
      <UButton
        color="neutral"
        variant="subtle"
        :disabled="!isNew && !dirty"
        @click="isNew ? router.push('/profiles') : (draft = initial())"
      >
        {{ isNew ? 'Cancel' : 'Revert' }}
      </UButton>
      <UButton
        icon="i-lucide-check"
        :disabled="!isNew && !dirty"
        @click="save"
      >
        {{ isNew ? 'Create profile' : 'Save changes' }}
      </UButton>
    </template>
  </PageLayout>
</template>
