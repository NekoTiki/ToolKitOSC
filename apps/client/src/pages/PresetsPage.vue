<script setup lang="ts">
import type { DropdownMenuItem } from '@nuxt/ui'
import IconSelectMenu from '@renderer/components/IconSelectMenu.vue'
import AvailableParametersList from '@renderer/components/preset-modal/AvailableParametersList.vue'
import CoverageBanner from '@renderer/components/preset-modal/CoverageBanner.vue'
import ParameterRow from '@renderer/components/preset-modal/ParameterRow.vue'
import EmptyState from '@renderer/components/ui/EmptyState.vue'
import FormSection from '@renderer/components/ui/FormSection.vue'
import SegmentedControl from '@renderer/components/ui/SegmentedControl.vue'
import { useAreYouSureModal } from '@renderer/composables/useAreYouSureModal'
import { useAvatarDetails } from '@renderer/composables/useAvatarDetails'
import { usePresets } from '@renderer/composables/usePresets'
import { api } from '@renderer/lib/tauri-bridge'
import { timeAgo } from '@renderer/utils/time'
import type { Preset, PresetParameter } from '@vrc-osc-toolkit/shared-ui'
import _ from 'lodash'
import { computed, nextTick, onMounted, ref, useTemplateRef, watch } from 'vue'
import { onBeforeRouteLeave, useRoute, useRouter } from 'vue-router'

// Presets: saved snapshots of parameter values, applied in one tap (from here or from a Preset
// control). The list lives in the sidebar; the selected preset opens in the main pane as an
// editor. "New from current state" pulls every value from VRChat first, so a new preset starts
// complete. Replaces the preset list and preset modals.
const route = useRoute()
const router = useRouter()
const toast = useToast()
const { avatarDetails } = useAvatarDetails()
const { presets, getPreset, addPreset, updatePreset, deletePreset, applyPreset, captureParameters, coverage, includedParameters } = usePresets()
const { openModal: confirm } = useAreYouSureModal()

const isNew = route.name === 'preset-new'
const presetId = computed(() => (typeof route.params.presetId === 'string' ? route.params.presetId : null))
const preset = computed(() => (presetId.value ? getPreset(presetId.value) : undefined))

// /presets alone opens the first preset, if any.
watch(
  [presets, presetId],
  () => {
    if (route.name !== 'presets') return
    if (presetId.value && !preset.value && presets.value.length) void router.replace(`/presets/${presets.value[0]!.id}`)
    else if (!presetId.value && presets.value.length) void router.replace(`/presets/${presets.value[0]!.id}`)
  },
  { immediate: true }
)

interface Draft {
  name: string
  icon?: string
  parameters: PresetParameter[]
}

const initial = (): Draft | null => {
  if (isNew) return { name: '', icon: '', parameters: captureParameters() }
  if (!preset.value) return null

  return { name: preset.value.name, icon: preset.value.icon, parameters: _.cloneDeep(preset.value.parameters) }
}

const draft = ref<Draft | null>(initial())
let snapshot = JSON.stringify(draft.value)

// The preset list loads asynchronously per avatar, so a direct link can arrive before it does.
watch(preset, (value) => {
  if (!draft.value && value) {
    draft.value = initial()
    snapshot = JSON.stringify(draft.value)
  }
})

const dirty = computed(() => !!draft.value && JSON.stringify(draft.value) !== snapshot)
const nameError = ref<string>()
const saved = ref(false)

// Merge freshly captured values in: adds new parameters and refreshes values, but never drops ones
// the user added by hand.
const captureIntoDraft = (): void => {
  if (!draft.value) return

  const byAddress = new Map(draft.value.parameters.map((param) => [param.address, param]))

  captureParameters().forEach((param) => byAddress.set(param.address, param))
  draft.value.parameters = Array.from(byAddress.values())
}

const pulling = ref(false)

// Best effort: if VRChat doesn't answer OSCQuery, the preset just starts from what's already
// been received.
const pullAndCapture = async (): Promise<void> => {
  pulling.value = true

  try {
    await api.forcePullParameters(false)
  } catch {
    // ignored, see above
  } finally {
    captureIntoDraft()
    pulling.value = false
  }
}

onMounted(() => {
  if (isNew) {
    void pullAndCapture().then(() => (snapshot = JSON.stringify(draft.value)))
  }
})

const tab = ref<'in' | 'available'>('in')

const availableCount = computed(() => {
  const inPreset = new Set(draft.value?.parameters.map((param) => param.address) ?? [])

  return includedParameters.value.filter((param) => !inPreset.has(param.address)).length
})

const removeParameter = (address: string): void => {
  if (draft.value) draft.value.parameters = draft.value.parameters.filter((param) => param.address !== address)
}

const save = (): void => {
  if (!draft.value) return

  const name = draft.value.name.trim()

  if (!name) {
    nameError.value = 'Give the preset a name.'

    return
  }

  saved.value = true

  if (isNew) {
    const id = addPreset(name, draft.value.icon, draft.value.parameters)

    toast.add({ title: `Created "${name}"`, icon: 'i-lucide-check', color: 'success' })
    void router.push(id ? `/presets/${id}` : '/presets')

    return
  }

  updatePreset(presetId.value!, { name, icon: draft.value.icon, parameters: draft.value.parameters })
  snapshot = JSON.stringify(draft.value)
  saved.value = false
  toast.add({ title: `Saved "${name}"`, icon: 'i-lucide-check', color: 'success' })
}

const apply = (): void => {
  if (!presetId.value) return

  applyPreset(presetId.value)
  toast.add({ title: 'Preset applied', icon: 'i-lucide-play', color: 'success' })
}

const remove = async (): Promise<void> => {
  if (!preset.value) return

  const ok = await confirm({
    title: 'Delete preset?',
    message: `Delete **${preset.value.name}**? Preset controls that use it become unavailable.`,
    confirmText: 'Delete'
  })

  if (!ok) return

  deletePreset(preset.value.id)
  saved.value = true
  void router.push('/presets')
}

// The name field, focused by the Rename menu item (directly, or after opening the preset with
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

const duplicate = (item: Preset): void => {
  const id = addPreset(`${item.name} copy`, item.icon, _.cloneDeep(item.parameters))

  if (!id) return

  toast.add({ title: `Duplicated "${item.name}"`, icon: 'i-lucide-copy' })
  void router.push(`/presets/${id}`)
}

const removeFromMenu = async (item: Preset): Promise<void> => {
  if (item.id === presetId.value) return remove()

  const ok = await confirm({
    title: 'Delete preset?',
    message: `Delete **${item.name}**? Preset controls that use it become unavailable.`,
    confirmText: 'Delete'
  })

  if (ok) deletePreset(item.id)
}

// A preset's actions, from its row in the sidebar: right-click it, or press "⋯" on the open one
// (VR pointers have no right-click).
const presetMenu = (item: Preset): DropdownMenuItem[][] => [
  [
    {
      label: 'Apply now',
      icon: 'i-lucide-play',
      onSelect: () => {
        applyPreset(item.id)
        toast.add({ title: `Applied "${item.name}"`, icon: 'i-lucide-play', color: 'success' })
      }
    },
    {
      label: 'Rename',
      icon: 'i-lucide-pen',
      onSelect: () => (item.id === presetId.value ? void focusName() : void router.push({ path: `/presets/${item.id}`, query: { rename: '1' } }))
    },
    { label: 'Duplicate', icon: 'i-lucide-copy', onSelect: () => duplicate(item) }
  ],
  [{ label: 'Delete', icon: 'i-lucide-trash-2', color: 'error', onSelect: () => void removeFromMenu(item) }]
]

onBeforeRouteLeave(async () => {
  if (saved.value || !dirty.value) return true

  return confirm({ title: 'Discard changes?', message: 'Your changes to this preset will be lost.', confirmText: 'Discard' })
})
</script>

<template>
  <PageLayout>
    <template #sidebar>
      <div class="flex min-h-9 shrink-0 items-center">
        <b class="text-xl font-semibold text-highlighted">Presets</b>
      </div>
      <UButton
        icon="i-lucide-camera"
        block
        :disabled="!avatarDetails"
        to="/presets/new"
      >
        New from current state
      </UButton>

      <nav
        aria-label="Presets"
        class="-mr-1.5 flex min-h-0 flex-1 flex-col gap-1.5 overflow-y-auto pr-1.5"
      >
        <UContextMenu
          v-for="item in presets"
          :key="item.id"
          :items="presetMenu(item)"
        >
          <RouterLink
            :to="`/presets/${item.id}`"
            class="grid min-h-14.5 grid-cols-[auto_minmax(0,1fr)_auto] items-center gap-x-2.5 gap-y-0.5 rounded-2xl border px-2.5 py-2 transition-colors"
            :class="item.id === presetId ? 'border-primary/50 bg-primary/14' : 'border-transparent hover:bg-(--aurora-glass)'"
          >
            <span
              class="row-span-2 grid size-8.5 place-items-center rounded-xl bg-(--aurora-well)"
              :class="item.id === presetId ? 'text-primary' : 'text-muted'"
            >
              <UIcon
                :name="item.icon || 'i-lucide-layers'"
                class="size-4.5"
              />
            </span>
            <b class="truncate text-[14.5px] font-semibold text-highlighted">{{ item.name }}</b>
            <!-- .prevent: the menu button sits inside the link, and must not follow it. -->
            <UDropdownMenu
              v-if="item.id === presetId"
              :items="presetMenu(item)"
              :content="{ align: 'start', side: 'right' }"
            >
              <UButton
                icon="i-lucide-ellipsis"
                color="neutral"
                variant="ghost"
                size="md"
                class="row-span-2"
                :aria-label="`${item.name} actions`"
                @click.prevent.stop
              />
            </UDropdownMenu>
            <small class="col-start-2 truncate text-xs text-muted">{{ item.parameters.length }} parameters · {{ timeAgo(item.updatedAt) }}</small>
          </RouterLink>
        </UContextMenu>
        <p
          v-if="!presets.length"
          class="px-1 text-sm text-muted"
        >
          No presets for this avatar yet.
        </p>
      </nav>

      <RouterLink
        to="/parameters?filter=excluded"
        class="flex min-h-14 shrink-0 items-center gap-2.5 rounded-2xl px-2.5 text-sm transition-colors hover:bg-(--aurora-glass)"
      >
        <span class="grid size-8.5 place-items-center rounded-xl bg-(--aurora-well) text-muted">
          <UIcon
            name="i-lucide-eye-off"
            class="size-4.5"
          />
        </span>
        <span class="grid min-w-0">
          <b class="font-semibold text-highlighted">Excluded parameters</b>
          <small class="text-xs text-muted">Hidden from new presets</small>
        </span>
      </RouterLink>
    </template>

    <template
      v-if="draft"
      #header
    >
      <PageHeader
        :title="isNew ? 'New preset' : preset?.name ?? 'Preset'"
        :meta="isNew ? 'Captured from your avatar just now' : preset ? `Updated ${timeAgo(preset.updatedAt)}` : undefined"
        :crumbs="[{ label: 'Presets', to: '/presets' }, { label: isNew ? 'New' : preset?.name ?? '' }]"
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
            icon="i-lucide-play"
            @click="apply"
          >
            Apply now
          </UButton>
        </template>
      </PageHeader>
    </template>

    <EmptyState
      v-if="!avatarDetails"
      icon="i-lucide-radio-tower"
      title="VRChat not detected"
      description="Presets belong to an avatar. Load into VRChat with OSC on and your avatar's presets show up here."
    />

    <EmptyState
      v-else-if="isNew && pulling"
      icon="i-lucide-loader-circle"
      title="Fetching current values from VRChat…"
      description="Reading every parameter so the preset starts complete."
      class="[&>span:first-child]:animate-spin"
    />

    <div
      v-else-if="draft"
      class="grid max-w-4xl grid-cols-1 gap-3.5"
    >
      <FormSection>
        <div class="grid gap-3 sm:grid-cols-2">
          <UFormField
            label="Name"
            required
            :error="nameError"
          >
            <UInput
              ref="nameInput"
              v-model="draft.name"
              placeholder="Default look, Party mode…"
              class="w-full"
              @update:model-value="nameError = undefined"
            />
          </UFormField>
          <UFormField label="Icon">
            <IconSelectMenu
              v-model="draft.icon"
              class="w-full"
              @keyup.backspace="draft.icon = ''"
            />
          </UFormField>
        </div>
      </FormSection>

      <CoverageBanner
        :captured="coverage.captured"
        :total="coverage.total"
      />

      <FormSection
        title="Parameters"
        :description="`${draft.parameters.length} in this preset. Capturing adds new ones and refreshes values; it never removes ones you added.`"
      >
        <template #actions>
          <UButton
            icon="i-lucide-camera"
            color="neutral"
            variant="subtle"
            :loading="pulling"
            @click="pullAndCapture"
          >
            Capture current state
          </UButton>
        </template>

        <SegmentedControl
          v-model="tab"
          class="w-fit"
          aria-label="Parameter list"
          :items="[
            { value: 'in', label: `In preset · ${draft.parameters.length}` },
            { value: 'available', label: `Available · ${availableCount}` }
          ]"
        />

        <template v-if="tab === 'in'">
          <p
            v-if="!draft.parameters.length"
            class="text-sm text-muted"
          >
            No parameters yet. Capture the current state, or add some from Available.
          </p>
          <!-- Virtualized: a preset can hold every parameter an avatar declares. Row spacing comes
          from each item's own padding - virtualized rows are absolutely positioned, so a gap on
          the container wouldn't apply. -->
          <UScrollArea
            v-else
            :items="draft.parameters"
            virtualize
            class="h-104"
            :ui="{ item: 'pb-2 last:pb-0' }"
          >
            <template #default="{ item: parameter, index }">
              <ParameterRow
                v-model="draft.parameters[index]!"
                @remove="removeParameter(parameter.address)"
              />
            </template>
          </UScrollArea>
        </template>

        <AvailableParametersList
          v-else
          v-model:parameters="draft.parameters"
        />
      </FormSection>
    </div>

    <EmptyState
      v-else
      icon="i-lucide-layers"
      title="No presets yet"
      description="A preset saves the current value of your avatar's parameters, so you can bring a whole look back in one tap."
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

    <template
      v-if="draft && avatarDetails && !(isNew && pulling)"
      #footer
    >
      <UButton
        color="neutral"
        variant="subtle"
        :disabled="!isNew && !dirty"
        @click="isNew ? router.push('/presets') : (draft = initial())"
      >
        {{ isNew ? 'Cancel' : 'Revert' }}
      </UButton>
      <UButton
        icon="i-lucide-check"
        :disabled="!isNew && !dirty"
        @click="save"
      >
        {{ isNew ? 'Create preset' : 'Save changes' }}
      </UButton>
    </template>
  </PageLayout>
</template>
