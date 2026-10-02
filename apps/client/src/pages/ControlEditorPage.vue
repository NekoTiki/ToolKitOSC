<script setup lang="ts">
import AddressField from '@renderer/components/control-modal/AddressField.vue'
import BooleanEnumFields from '@renderer/components/control-modal/BooleanEnumFields.vue'
import BooleanGroupFields from '@renderer/components/control-modal/BooleanGroupFields.vue'
import EnumOptionsField from '@renderer/components/control-modal/EnumOptionsField.vue'
import IntifaceFields from '@renderer/components/control-modal/IntifaceFields.vue'
import IntifacePatternFields from '@renderer/components/control-modal/IntifacePatternFields.vue'
import OpenShockFields from '@renderer/components/control-modal/OpenShockFields.vue'
import PresetField from '@renderer/components/control-modal/PresetField.vue'
import IconSelectMenu from '@renderer/components/IconSelectMenu.vue'
import ChoiceCard from '@renderer/components/ui/ChoiceCard.vue'
import FormSection from '@renderer/components/ui/FormSection.vue'
import SegmentedControl from '@renderer/components/ui/SegmentedControl.vue'
import { useAreYouSureModal } from '@renderer/composables/useAreYouSureModal'
import { useControlEditor } from '@renderer/composables/useControlEditor'
import { useControls } from '@renderer/composables/useControls'
import { useControlsView } from '@renderer/composables/useControlsView'
import type {
  BooleanEnumControl,
  BooleanGroupControl,
  ControlType,
  EnumControl,
  IntifacePatternControl,
  IntifaceToyControl
} from '@toolkitosc/shared-ui'
import { Control, CONTROL_TYPE_LABELS, controlTileSpan } from '@toolkitosc/shared-ui'
import { computed, onMounted, ref } from 'vue'
import { onBeforeRouteLeave, useRoute, useRouter } from 'vue-router'

// Create or edit one control, as a full page instead of the old modal: type cards, basics, the
// type's own settings, and a live preview tile on the right that really sends commands, so the
// control can be tested before saving. Reached from "Add control" or a tile's Edit button.
const route = useRoute()
const router = useRouter()
const toast = useToast()
const { controls, handleCommand, deleteControl } = useControls()
const { density } = useControlsView()
const { openModal: confirm } = useAreYouSureModal()

const groupId = String(route.params.groupId)
const controlId = route.name === 'control-new' ? null : String(route.params.controlId)

const editor = useControlEditor(groupId, controlId)
const { model, type, targetGroupId, errors, addresses, showAllAddresses, presetId, shockerList } = editor

const group = computed(() => controls.value.find((g) => g.id === groupId))

// An id that no longer exists (deleted elsewhere, a different avatar loaded) has nothing to edit.
onMounted(() => {
  if (!group.value || (controlId && editor.isNew)) void router.replace(`/controls/${group.value ? groupId : ''}`)
})

interface TypeChoice {
  value: ControlType['type']
  icon: string
  description: string
  disabled?: boolean
  reason?: string
}

const typeGroups = computed<{ label: string; items: TypeChoice[] }[]>(() => [
  {
    label: 'VRChat',
    items: [
      { value: 'boolean', icon: 'i-lucide-toggle-right', description: 'On or off, one parameter' },
      { value: 'boolean-group', icon: 'i-lucide-layers', description: 'One switch for several parameters' },
      { value: 'boolean-enum', icon: 'i-lucide-git-fork', description: 'Options that set several toggles' },
      { value: 'enum', icon: 'i-lucide-list', description: 'Pick one value from a list' },
      { value: 'step-enum', icon: 'i-lucide-chevrons-left-right', description: 'Previous / next through a list' },
      { value: 'slider', icon: 'i-lucide-sliders-horizontal', description: 'A value from 0 to 100%' }
    ]
  },
  {
    label: 'OpenShock',
    items: [
      {
        value: 'open-shock-shocker',
        icon: 'material-symbols:electric-bolt',
        description: 'A random shock or vibration in a range',
        disabled: !editor.openShockAvailable.value,
        reason: 'Add an OpenShock API key in Settings'
      }
    ]
  },
  {
    label: 'Intiface',
    items: [
      { value: 'intiface-toy', icon: 'mdi:vibrate', description: 'Vibration strength', disabled: !editor.intiface.available.value, reason: "Intiface isn't connected" },
      { value: 'intiface-pattern', icon: 'i-lucide-audio-waveform', description: 'Pick a vibration pattern', disabled: !editor.intiface.available.value, reason: "Intiface isn't connected" }
    ]
  },
  {
    label: 'Presets',
    items: [{ value: 'preset', icon: 'i-lucide-play', description: 'Apply a saved preset in one tap', disabled: !editor.hasPresets.value, reason: 'Create a preset first' }]
  }
])

// Option pickers size themselves from their option count (see shared-ui's controlTileSpan).
const sizeable = computed(() => !['enum', 'boolean-enum', 'intiface-pattern'].includes(model.value.type ?? ''))

const tileWidth = computed<'normal' | 'wide'>({
  get: () => (model.value.size === 'wide' ? 'wide' : 'normal'),
  set: (value) => (model.value.size = value === 'wide' ? 'wide' : undefined)
})

const groupItems = computed(() => controls.value.map((g) => ({ label: g.name, value: g.id })))

// The field components' v-models are required arrays; these give them one even on a fresh draft.
const booleanGroupInputs = computed<BooleanGroupControl['inputs']>({
  get: () => (model.value.inputs as BooleanGroupControl['inputs'] | undefined) ?? [],
  set: (value) => (model.value.inputs = value)
})
const booleanEnumInputs = computed<BooleanEnumControl['inputs']>({
  get: () => (model.value.inputs as BooleanEnumControl['inputs'] | undefined) ?? [],
  set: (value) => (model.value.inputs = value)
})
const enumOptions = computed<EnumControl['options']>({
  get: () => model.value.options ?? [],
  set: (value) => (model.value.options = value)
})
const actuators = computed<IntifaceToyControl['actuators']>({
  get: () => model.value.actuators ?? [],
  set: (value) => (model.value.actuators = value)
})
const allowedPatterns = computed<IntifacePatternControl['allowedPatterns']>({
  get: () => model.value.allowedPatterns ?? [],
  set: (value) => (model.value.allowedPatterns = value)
})

const openShockModes = [
  { label: 'Shock', value: 'Shock' as const },
  { label: 'Vibrate', value: 'Vibrate' as const }
]

// Large tiles don't fit two-wide in the preview column, and a 2×2 picker previews as one row
// (its options scroll), so the preview is capped at medium.
const previewDensity = computed(() => (density.value === 'l' ? 'm' : density.value))
const previewSpan = computed(() => (controlTileSpan(preview.value) === 's' ? 's' : 'm'))

const preview = computed(() => ({ ...model.value, id: model.value.id ?? 'preview', name: model.value.name || 'Untitled control' }) as ControlType)

const saved = ref(false)

const backTo = computed(() => `/controls/${targetGroupId.value}`)

const save = (): void => {
  if (!editor.save()) {
    toast.add({ title: 'Fix the highlighted fields', icon: 'i-lucide-triangle-alert', color: 'warning' })

    return
  }

  saved.value = true
  toast.add({ title: editor.isNew ? `Created "${model.value.name}"` : `Saved "${model.value.name}"`, icon: 'i-lucide-check', color: 'success' })
  void router.push(backTo.value)
}

const remove = async (): Promise<void> => {
  if (!model.value.id) return

  const ok = await confirm({ title: 'Delete control?', message: `Delete **${model.value.name}**? This can't be undone.`, confirmText: 'Delete' })

  if (!ok) return

  deleteControl(groupId, model.value.id)
  saved.value = true
  void router.push(`/controls/${groupId}`)
}

onBeforeRouteLeave(async () => {
  if (saved.value || !editor.dirty.value) return true

  return confirm({ title: 'Discard changes?', message: 'Your changes to this control will be lost.', confirmText: 'Discard' })
})
</script>

<template>
  <PageLayout v-if="group">
    <template #header>
      <PageHeader
        :title="editor.isNew ? 'New control' : 'Edit control'"
        :crumbs="[
          { label: 'Controls', to: `/controls/${groupId}` },
          { label: group.name, to: `/controls/${groupId}` },
          { label: editor.isNew ? 'New' : model.name || 'Untitled' }
        ]"
      >
        <template
          v-if="!editor.isNew"
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
        </template>
      </PageHeader>
    </template>

    <div class="grid items-start gap-4.5 lg:grid-cols-[minmax(0,1fr)_26rem]">
      <div class="grid min-w-0 gap-3.5">
        <FormSection title="Type">
          <template
            v-for="typeGroup in typeGroups"
            :key="typeGroup.label"
          >
            <p class="-mb-1 font-mono text-[10.5px] tracking-widest text-muted uppercase">
              {{ typeGroup.label }}
            </p>
            <div class="grid grid-cols-[repeat(auto-fill,minmax(10.5rem,1fr))] gap-2.5">
              <ChoiceCard
                v-for="choice in typeGroup.items"
                :key="choice.value"
                :title="CONTROL_TYPE_LABELS[choice.value]"
                :description="choice.description"
                :icon="choice.icon"
                :selected="type === choice.value"
                :disabled="choice.disabled && type !== choice.value"
                :disabled-reason="choice.reason"
                @click="type = choice.value"
              />
            </div>
          </template>
        </FormSection>

        <FormSection title="Basics">
          <UFormField
            label="Name"
            required
            :error="errors.name"
          >
            <UInput
              v-model="model.name"
              placeholder="Hoodie, Face, Tail wag…"
              class="w-full"
              @update:model-value="errors.name = undefined"
            />
          </UFormField>

          <UFormField
            v-if="type !== 'open-shock-shocker'"
            label="Icon"
          >
            <IconSelectMenu
              v-model="model.icon"
              class="w-full"
              @keyup.backspace="model.icon = ''"
            />
          </UFormField>

          <div class="grid gap-3 sm:grid-cols-2">
            <UFormField
              label="Group"
              :description="targetGroupId !== groupId ? 'The control moves to this group when you save.' : undefined"
            >
              <USelect
                v-model="targetGroupId"
                :items="groupItems"
                class="w-full"
              />
            </UFormField>
            <UFormField
              label="Tile width"
              :description="sizeable ? undefined : 'Automatic: two cells, or 2×2 above 8 options.'"
            >
              <SegmentedControl
                v-if="sizeable"
                v-model="tileWidth"
                aria-label="Tile width"
                class="w-fit"
                :items="[
                  { value: 'normal', label: '1 cell' },
                  { value: 'wide', label: '2 cells' }
                ]"
              />
            </UFormField>
          </div>

          <USwitch
            :model-value="!!model.pinned"
            label="Pin this control"
            description="Pinned controls are listed together at the top of the Controls page."
            @update:model-value="model.pinned = $event || undefined"
          />
        </FormSection>

        <FormSection :title="`${type ? CONTROL_TYPE_LABELS[type] : ''} settings`">
          <template v-if="type === 'boolean' || type === 'enum' || type === 'slider' || type === 'step-enum'">
            <UFormField :error="errors.address">
              <AddressField
                v-model:address="model.inputAddress"
                v-model:show-all="showAllAddresses"
                :addresses="addresses"
              />
            </UFormField>
            <UCheckbox
              v-if="type === 'boolean'"
              v-model="model.reverse"
              label="Reverse"
              description="On sends false, off sends true."
            />
          </template>

          <UFormField
            v-if="type === 'boolean-group'"
            :error="errors.inputs"
          >
            <BooleanGroupFields
              v-model:inputs="booleanGroupInputs"
              v-model:show-all="showAllAddresses"
              :addresses="addresses"
            />
          </UFormField>

          <UFormField
            v-if="type === 'boolean-enum'"
            :error="errors.inputs"
          >
            <BooleanEnumFields
              v-model:inputs="booleanEnumInputs"
              v-model:show-all="showAllAddresses"
              :addresses="addresses"
            />
          </UFormField>

          <UFormField
            v-if="type === 'enum' || type === 'step-enum'"
            label="Options"
            required
            :error="errors.options"
            :ui="{ container: 'grid gap-2' }"
          >
            <EnumOptionsField v-model="enumOptions" />
          </UFormField>

          <template v-if="type === 'open-shock-shocker'">
            <UAlert
              v-if="!editor.openShockAvailable.value"
              color="warning"
              variant="subtle"
              icon="i-lucide-shield-off"
              title="OpenShock unavailable"
              description="No valid OpenShock API key is set. Add one in Settings; until then this control is unavailable to everyone."
            />
            <UFormField :error="errors.shockers ?? errors.range">
              <OpenShockFields
                v-model:mode="model.mode"
                v-model:shockers="model.shockers"
                v-model:intensity="model.intensity"
                v-model:duration="model.duration"
                v-model:cooldown="model.cooldown"
                :open-shock-mode="openShockModes"
                :shocker-list="shockerList"
              />
            </UFormField>
          </template>

          <template v-if="type === 'intiface-toy' || type === 'intiface-pattern'">
            <UAlert
              v-if="!editor.intiface.available.value"
              color="warning"
              variant="subtle"
              icon="i-lucide-shield-off"
              title="Intiface unavailable"
              description="Not connected to Intiface. Turn it on in Settings; until then this control is unavailable to everyone."
            />
            <UFormField :error="errors.actuators">
              <IntifaceFields
                v-model:actuators="actuators"
                :devices="editor.intiface.devices.value"
              />
            </UFormField>
            <UFormField
              v-if="type === 'intiface-pattern'"
              :error="errors.patterns"
            >
              <IntifacePatternFields v-model:allowed-patterns="allowedPatterns" />
            </UFormField>
          </template>

          <UFormField
            v-if="type === 'preset'"
            :error="errors.preset"
          >
            <PresetField v-model="presetId" />
          </UFormField>
        </FormSection>
      </div>

      <aside class="sticky top-0 grid gap-3.5">
        <FormSection>
          <p class="font-mono text-[10.5px] tracking-widest text-muted uppercase">
            Live preview
          </p>
          <div
            class="tile-grid justify-center"
            :data-density="previewDensity"
            :style="{ gridTemplateColumns: `repeat(${previewSpan === 's' ? 1 : 2}, var(--tile-cell))` }"
          >
            <div :data-span="previewSpan">
              <Control
                :control="preview"
                :unavailable="editor.previewUnavailable.value"
                @command="handleCommand(preview, $event)"
              />
            </div>
          </div>
          <p class="text-center text-[13px] text-muted">
            Using the preview sends real commands to VRChat.
          </p>
        </FormSection>
      </aside>
    </div>

    <template #footer>
      <p class="mr-auto flex min-w-0 items-center gap-1.5 text-[12.5px] text-muted">
        <UIcon
          name="i-lucide-info"
          class="size-4 shrink-0"
        />
        {{ editor.isNew ? 'Viewers see the new control when you save.' : 'Viewers see your changes when you save.' }}
      </p>
      <UButton
        color="neutral"
        variant="subtle"
        :to="`/controls/${groupId}`"
      >
        Cancel
      </UButton>
      <UButton
        icon="i-lucide-check"
        @click="save"
      >
        {{ editor.isNew ? 'Create control' : 'Save changes' }}
      </UButton>
    </template>
  </PageLayout>
</template>
