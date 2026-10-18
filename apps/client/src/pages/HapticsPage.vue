<script setup lang="ts">
import BhapticsMotors from '@renderer/components/bhaptics/BhapticsMotors.vue'
import HapticsSourceEditor from '@renderer/components/haptics/HapticsSourceEditor.vue'
import OwoMuscles from '@renderer/components/owo/OwoMuscles.vue'
import EmptyState from '@renderer/components/ui/EmptyState.vue'
import FormSection from '@renderer/components/ui/FormSection.vue'
import SegmentedControl from '@renderer/components/ui/SegmentedControl.vue'
import { useAddons } from '@renderer/composables/useAddons'
import { useAreYouSureModal } from '@renderer/composables/useAreYouSureModal'
import type { HapticsToy } from '@renderer/composables/useAvatarHaptics'
import {
  newSource,
  SOURCE_CONTACTS,
  SOURCE_ZONE_KIND,
  useAvatarHaptics,
  ZONE_KIND_LABEL} from '@renderer/composables/useAvatarHaptics'
import { useBhaptics } from '@renderer/composables/useBhaptics'
import { isTouched } from '@renderer/composables/useIntiface'
import { useOwo } from '@renderer/composables/useOwo'
import type { HapticsBhapticsTarget, HapticsSource } from '@renderer/lib/tauri-bridge'
import { computed, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'

// Haptics: which toy follows which part of your avatar. Each toy motor, and each part of your
// bHaptics gear, is in the sidebar with a live meter; the main pane lists its sources (each opens
// its own page) and its Max strength, for every avatar or only the one you wear. Touch priority has
// its own pane.
const route = useRoute()
const router = useRouter()
const { openModal: confirm } = useAreYouSureModal()
const {
  settings,
  state,
  scope,
  avatarId,
  avatarName,
  avatar,
  toys,
  zones,
  toyName,
  globalOutput,
  avatarOutput,
  activeOutput,
  editedOutput,
  level,
  change,
  outputToEdit,
  removeOverride
} = useAvatarHaptics()
const { isOn } = useAddons()
const { state: bhapticsState } = useBhaptics()
const { state: owoState } = useOwo()

// The sidebar's lists: toys, parts of your bHaptics gear, then your OWO Skin's muscles.
const toyGroups = computed(() =>
  [
    { key: 'toys', label: 'Toys', items: toys.value.filter((item) => !item.bhaptics && !item.owo), empty: isOn('intiface') ? 'No toys yet. Connect one in Intiface Central.' : null },
    { key: 'bhaptics', label: 'bHaptics', items: toys.value.filter((item) => item.bhaptics), empty: isOn('bhaptics') ? 'Nothing connected. Pair your gear in bHaptics Player.' : null },
    { key: 'owo', label: 'OWO', items: toys.value.filter((item) => item.owo), empty: isOn('owo') ? 'Not connected. Connect ToolKitOSC in the OWO app.' : null }
  ].filter((group) => group.items.length || group.empty)
)

const isPriority = route.name === 'haptics-priority'
const toyKey = computed(() => (typeof route.params.actuator === 'string' ? route.params.actuator : null))
const toy = computed(() => toys.value.find((item) => item.key === toyKey.value))
// /haptics/<toy>/sources/<n>: one of the toy's sources, as a sub-page of the toy.
const sourceIndex = route.name === 'haptics-source' ? Number(route.params.index) : null

// /haptics alone (or a toy that's gone) opens the first toy, or touch priority without any.
watch(
  [toys, toyKey],
  () => {
    if (isPriority || ((route.name === 'haptics-toy' || route.name === 'haptics-source') && toy.value)) return

    void router.replace(toys.value[0] ? `/haptics/${toys.value[0].key}` : '/haptics/priority')
  },
  { immediate: true }
)

if (!avatarId.value) scope.value = 'global'

const scopeItems = computed(() => [
  { value: 'global' as const, label: 'All avatars' },
  ...(avatarId.value ? [{ value: 'avatar' as const, label: `${avatarName.value} only` }] : [])
])

const pct = (value: number): string => `${Math.round(value * 100)} %`

const sourceCount = (item: HapticsToy): string => {
  const count = activeOutput(item.key)?.sources.length ?? 0

  return count === 0 ? 'no sources' : count === 1 ? '1 source' : `${count} sources`
}

const output = computed(() => (toy.value ? editedOutput(toy.value.key) : undefined))
// The levels Rust reports are for what applies on the worn avatar, so they're only shown for that.
const live = computed(() => !!toy.value && !!output.value && output.value === activeOutput(toy.value.key))
// On the worn avatar without its own setup for this toy, the one for every avatar applies.
const inherited = computed(() => scope.value === 'avatar' && !!toy.value && !avatarOutput(toy.value.key))
const shown = computed(() => (inherited.value && toy.value ? globalOutput(toy.value.key) : output.value))

const sourceTitle = (source: HapticsSource): string => {
  const names = source.zones.include.filter((name) => name.trim())
  const kind = ZONE_KIND_LABEL[SOURCE_ZONE_KIND[source.kind]]

  return names.length ? names.join(', ') : `Every ${kind.toLowerCase()}`
}

const sourceSummary = (source: HapticsSource): string => {
  const on = SOURCE_CONTACTS[source.kind]
    .filter((contact) => (source as unknown as Record<string, boolean>)[contact.key])
    .map((contact) => contact.label.toLowerCase())
  const extras = [
    source.shape.followMovement ? 'follows movement' : null,
    source.shape.deadZone > 0 ? `dead zone ${pct(source.shape.deadZone)}` : null,
    source.shape.scale < 1 ? `max ${pct(source.shape.scale)}` : null
  ].filter(Boolean)

  return [ZONE_KIND_LABEL[SOURCE_ZONE_KIND[source.kind]], on.length ? on.join(' · ') : 'reacts to nothing', ...extras].join(' · ')
}

const addSource = (kind: HapticsSource['kind']): void => {
  if (!toy.value) return

  const current = toy.value
  let at = 0

  change((target) => {
    const edited = outputToEdit(target, current)
    edited.sources.push(newSource(kind))
    at = edited.sources.length - 1
  })
  void router.push(`/haptics/${current.key}/sources/${at}`)
}

const addItems = [
  [
    { label: 'Socket', icon: 'i-lucide-circle-dot', onSelect: () => addSource('sps-socket') },
    { label: 'Plug', icon: 'i-lucide-arrow-up-right', onSelect: () => addSource('sps-plug') },
    { label: 'Touch zone', icon: 'i-lucide-hand', onSelect: () => addSource('sps-touch') }
  ]
]

const setMaxStrength = (value: number | undefined): void => {
  if (!toy.value || value === undefined) return

  const current = toy.value
  change((target) => (outputToEdit(target, current).maxStrength = value / 100))
}

const ownSetup = (): void => {
  if (!toy.value) return

  const current = toy.value
  change((target) => outputToEdit(target, current))
}

// A part of your bHaptics gear: how its level lights it.
const fill = computed(() => shown.value?.actuator.bhaptics?.fill ?? 'all')

const setFill = (value: HapticsBhapticsTarget['fill']): void => {
  if (!toy.value?.bhaptics) return

  const current = toy.value
  change((target) => {
    const edited = outputToEdit(target, current)
    edited.actuator.bhaptics = { ...current.bhaptics!, ...edited.actuator.bhaptics, fill: value }
  })
}

const editedSource = computed(() => (sourceIndex !== null && toy.value ? editedOutput(toy.value.key)?.sources[sourceIndex] : undefined))

const removeSource = async (): Promise<void> => {
  if (!toy.value || !editedSource.value || sourceIndex === null) return

  const current = toy.value
  const kind = ZONE_KIND_LABEL[SOURCE_ZONE_KIND[editedSource.value.kind]].toLowerCase()
  const ok = await confirm({ title: 'Delete source?', message: `This toy stops following this ${kind}.`, confirmText: 'Delete' })

  if (!ok) return

  change((target) => outputToEdit(target, current).sources.splice(sourceIndex, 1))
  void router.push(`/haptics/${current.key}`)
}

const setAvatarEnabled = (enabled: boolean): void =>
  change((target) => {
    if (!avatarId.value) return

    const own = (target.avatars[avatarId.value] ??= { name: avatarName.value, enabled: true, outputs: [] })
    own.name = avatarName.value
    own.enabled = enabled
  })
</script>

<template>
  <PageLayout>
    <template #sidebar>
      <div class="flex min-h-9 shrink-0 items-center gap-2">
        <b class="flex-1 text-xl font-semibold text-highlighted">Haptics</b>
      </div>

      <div
        v-if="settings"
        class="grid gap-1.5"
      >
        <div class="flex items-center gap-3 rounded-field bg-(--aurora-well) px-3 py-2.5">
          <span class="grid min-w-0 flex-1">
            <b class="text-[14.5px] font-semibold text-highlighted">Avatar haptics</b>
            <small class="text-xs text-muted">Also <code>TKOSC/Haptics</code></small>
          </span>
          <USwitch
            :model-value="settings.enabled"
            aria-label="Avatar haptics"
            @update:model-value="change((target) => (target.enabled = $event))"
          />
        </div>
        <RouterLink
          to="/haptics/priority"
          class="flex items-center gap-3 rounded-field border px-3 py-2.5 transition-colors"
          :class="isPriority ? 'border-primary/50 bg-primary/14' : 'border-transparent bg-(--aurora-well) hover:bg-(--aurora-glass)'"
        >
          <span class="grid min-w-0 flex-1">
            <b class="text-[14.5px] font-semibold text-highlighted">Touch priority</b>
            <small class="text-xs text-muted">Above {{ pct(settings.priority.threshold) }} · grace {{ (settings.priority.graceMs / 1000).toFixed(1) }} s</small>
          </span>
          <UIcon
            name="i-lucide-chevron-right"
            class="size-4 text-dimmed"
          />
        </RouterLink>
      </div>

      <nav
        aria-label="Toys"
        class="-mr-1.5 flex min-h-0 flex-1 flex-col gap-1.5 overflow-y-auto pr-1.5"
      >
        <template
          v-for="group in toyGroups"
          :key="group.key"
        >
          <small class="px-1.5 pt-2 text-xs font-medium text-muted">{{ group.label }}</small>
          <RouterLink
            v-for="item in group.items"
            :key="item.key"
            :to="`/haptics/${item.key}`"
            class="grid gap-1.5 rounded-field border px-3 py-2.5 transition-colors"
            :class="item.key === toyKey ? 'border-primary/50 bg-primary/14' : 'border-transparent hover:bg-(--aurora-glass)'"
          >
            <span class="flex min-w-0 items-center gap-2">
              <b class="flex-1 truncate text-[14.5px] font-semibold text-highlighted">{{ toyName(item) }}</b>
              <span
                v-if="item.bhaptics ? bhapticsState.touched.includes(item.bhaptics.position) : item.owo ? owoState.touched.includes(item.owo.region.endsWith('back') ? 'back' : 'front') : isTouched([item])"
                class="shrink-0 rounded-full bg-secondary/15 px-2 py-0.5 text-[11.5px] font-medium text-secondary"
              >Touch</span>
              <span
                v-else-if="!item.connected"
                class="shrink-0 rounded-full bg-(--aurora-well) px-2 py-0.5 text-[11.5px] text-muted"
              >Offline</span>
            </span>
            <small class="truncate text-xs text-muted">{{ item.actuatorType }} · {{ sourceCount(item) }}</small>
            <span
              class="h-1.5 overflow-hidden rounded-full bg-(--aurora-well)"
              role="meter"
              :aria-valuenow="level(item.key).level"
              aria-valuemin="0"
              aria-valuemax="1"
              :aria-label="`${toyName(item)} level`"
            >
              <span
                class="block h-full rounded-full bg-secondary transition-[width] duration-100"
                :style="{ width: `${level(item.key).level * 100}%` }"
              />
            </span>
          </RouterLink>
          <p
            v-if="!group.items.length"
            class="px-1.5 text-sm text-muted"
          >
            {{ group.empty }}
          </p>
        </template>
      </nav>
    </template>

    <template
      v-if="isPriority || toy"
      #header
    >
      <PageHeader
        v-if="toy && sourceIndex !== null"
        :title="editedSource ? ZONE_KIND_LABEL[SOURCE_ZONE_KIND[editedSource.kind]] : 'Source'"
        :meta="scope === 'avatar' ? `${avatarName} only` : 'All avatars'"
        :crumbs="[
          { label: 'Haptics', to: '/haptics' },
          { label: toyName(toy), to: `/haptics/${toy.key}` },
          { label: editedSource ? ZONE_KIND_LABEL[SOURCE_ZONE_KIND[editedSource.kind]] : 'Source' }
        ]"
      >
        <template
          v-if="editedSource"
          #actions
        >
          <UButton
            icon="i-lucide-trash-2"
            color="error"
            variant="soft"
            @click="removeSource"
          >
            Delete
          </UButton>
        </template>
      </PageHeader>
      <PageHeader
        v-else
        :title="isPriority ? 'Touch priority' : toyName(toy!)"
        :meta="isPriority ? undefined : `${toy!.actuatorType}${toy!.connected ? '' : ' · offline'}`"
        :crumbs="[{ label: 'Haptics', to: '/haptics' }, { label: isPriority ? 'Touch priority' : toyName(toy!) }]"
      >
        <template
          v-if="toy && !inherited"
          #actions
        >
          <UDropdownMenu
            :items="addItems"
            :content="{ align: 'end' }"
          >
            <UButton icon="i-lucide-plus">
              Add source
            </UButton>
          </UDropdownMenu>
        </template>
      </PageHeader>
    </template>

    <UAlert
      v-if="settings && state.ogbHint"
      color="warning"
      variant="subtle"
      icon="i-lucide-triangle-alert"
      title="Is OscGoesBrrr still running?"
      description="Your avatar's contacts are moving, but no toy follows them here yet. Close OscGoesBrrr before you set one up, or both apps will drive the same toys."
      class="mb-3.5 max-w-3xl"
    />

    <EmptyState
      v-if="!settings"
      icon="i-lucide-loader-circle"
      title="Loading…"
      class="[&>span:first-child]:animate-spin"
    />

    <!-- Touch priority -->
    <div
      v-else-if="isPriority"
      class="grid max-w-3xl gap-3.5"
    >
      <FormSection
        title="Touch comes first"
        description="While someone touches your avatar, the toys it drives follow the touch. What viewers send keeps running underneath and the toy picks it up again once the touch is over. Toys without sources work as before."
      >
        <div class="grid gap-2">
          <div class="flex items-center justify-between gap-3 text-sm">
            <span class="text-muted">Touch takes over above</span>
            <b class="font-semibold text-highlighted tabular-nums">{{ pct(settings.priority.threshold) }}</b>
          </div>
          <USlider
            :model-value="Math.round(settings.priority.threshold * 100)"
            :min="0"
            :max="50"
            :step="1"
            size="lg"
            aria-label="Touch takes over above"
            @update:model-value="change((target) => (target.priority.threshold = ($event ?? 0) / 100))"
          />
        </div>
        <div class="grid gap-2">
          <div class="flex items-center justify-between gap-3 text-sm">
            <span class="text-muted">Viewers wait after the touch ends for</span>
            <b class="font-semibold text-highlighted tabular-nums">{{ (settings.priority.graceMs / 1000).toFixed(1) }} s</b>
          </div>
          <USlider
            :model-value="settings.priority.graceMs"
            :min="0"
            :max="10000"
            :step="100"
            size="lg"
            aria-label="Grace after touch"
            @update:model-value="change((target) => (target.priority.graceMs = $event ?? 0))"
          />
        </div>
      </FormSection>

      <FormSection
        title="Avatar parameters"
        description="Add these to your avatar to use them in VRChat."
      >
        <dl class="grid gap-2 text-sm">
          <div class="grid gap-0.5 rounded-field bg-(--aurora-well) px-3.5 py-2.5">
            <dt><code class="font-semibold text-highlighted">TKOSC/Haptics</code> <span class="text-muted">· bool</span></dt>
            <dd class="text-muted">
              Turns avatar haptics on and off from your expression menu, like the switch here.
            </dd>
          </div>
          <div class="grid gap-0.5 rounded-field bg-(--aurora-well) px-3.5 py-2.5">
            <dt><code class="font-semibold text-highlighted">TKOSC/HapticsLevel</code> <span class="text-muted">· float</span></dt>
            <dd class="text-muted">
              The strongest level touch drives right now, 0 to 1, for shaders or sounds.
            </dd>
          </div>
        </dl>
      </FormSection>
    </div>

    <!-- One of the toy's sources -->
    <HapticsSourceEditor
      v-else-if="toy && sourceIndex !== null"
      :toy="toy"
      :index="sourceIndex"
    />

    <!-- One toy -->
    <div
      v-else-if="toy"
      class="grid max-w-3xl gap-3.5"
    >
      <div class="flex flex-wrap items-center gap-3">
        <SegmentedControl
          v-model="scope"
          :items="scopeItems"
          aria-label="Which avatars"
        />
        <USwitch
          v-if="scope === 'avatar'"
          :model-value="avatar?.enabled ?? true"
          :label="`Haptics on ${avatarName}`"
          @update:model-value="setAvatarEnabled"
        />
      </div>

      <FormSection
        :title="avatarId ? `Wearing ${avatarName}` : 'No avatar yet'"
        :description="
          !avatarId
            ? 'Its zones show here once VRChat tells the app which avatar you wear.'
            : zones.length
              ? `${zones.length} zone${zones.length === 1 ? '' : 's'} found. They light up when touched.`
              : 'No SPS sockets, plugs or touch zones found on this avatar.'
        "
      >
        <div
          v-if="zones.length"
          class="flex flex-wrap gap-1.5"
        >
          <span
            v-for="zone in zones"
            :key="`${zone.kind}/${zone.name}`"
            class="relative isolate overflow-hidden rounded-full border px-2.5 py-1 text-xs transition-colors"
            :class="zone.level > 0 ? 'border-secondary/50 bg-(--aurora-well) text-secondary' : 'border-transparent bg-(--aurora-well) text-muted'"
          >
            <!-- How strongly it's touched, as a fill behind the name. -->
            <span
              class="absolute inset-y-0 left-0 -z-10 bg-secondary/25 transition-[width] duration-100"
              :style="{ width: `${zone.level * 100}%` }"
              aria-hidden="true"
            />
            {{ zone.name }}
            <span class="text-dimmed">· {{ ZONE_KIND_LABEL[zone.kind].toLowerCase() }}</span>
          </span>
        </div>
      </FormSection>

      <UAlert
        v-if="scope === 'avatar' && avatar && !avatar.enabled"
        color="neutral"
        variant="subtle"
        icon="i-lucide-pause"
        :title="`Haptics are off on ${avatarName}`"
        description="No toy follows this avatar until you turn it back on."
      />

      <FormSection
        v-if="inherited"
        title="Same as all avatars"
        :description="`${avatarName} uses the setup for all avatars on this toy. Give it its own to change it here only.`"
      >
        <div>
          <UButton
            icon="i-lucide-copy-plus"
            @click="ownSetup"
          >
            Set up for {{ avatarName }} only
          </UButton>
        </div>
      </FormSection>

      <FormSection
        title="Sources"
        :description="shown?.sources.length ? 'The toy follows the strongest one.' : 'Add a socket, plug or touch zone for this toy to follow.'"
      >
        <template
          v-if="scope === 'avatar' && !inherited"
          #actions
        >
          <UButton
            color="neutral"
            variant="subtle"
            icon="i-lucide-undo-2"
            @click="removeOverride(toy.key)"
          >
            Use the setup for all avatars
          </UButton>
        </template>
        <div
          v-if="shown?.sources.length"
          class="grid gap-1.5"
        >
          <component
            :is="inherited ? 'div' : 'RouterLink'"
            v-for="(source, index) in shown.sources"
            :key="index"
            :to="inherited ? undefined : `/haptics/${toy.key}/sources/${index}`"
            class="relative isolate flex min-h-14 items-center gap-3 overflow-hidden rounded-field bg-(--aurora-well) px-3.5 py-2.5"
            :class="{ 'transition-colors hover:bg-(--aurora-glass)': !inherited }"
          >
            <!-- The source's live level, as a fill behind the card. -->
            <span
              v-if="live || inherited"
              class="absolute inset-y-0 left-0 -z-10 bg-secondary/15 transition-[width] duration-100"
              :style="{ width: `${(level(toy.key).sources[index] ?? 0) * 100}%` }"
              aria-hidden="true"
            />
            <span class="grid min-w-0 flex-1">
              <b class="truncate text-[14.5px] font-semibold text-highlighted">{{ sourceTitle(source) }}</b>
              <small class="truncate text-xs text-muted">{{ sourceSummary(source) }}</small>
            </span>
            <span
              v-if="live || inherited"
              class="shrink-0 rounded-full px-2.5 py-1 text-xs font-medium tabular-nums"
              :class="(level(toy.key).sources[index] ?? 0) > 0 ? 'bg-secondary/15 text-secondary' : 'bg-(--aurora-glass) text-muted'"
            >{{ pct(level(toy.key).sources[index] ?? 0) }}</span>
            <UIcon
              v-if="!inherited"
              name="i-lucide-chevron-right"
              class="size-4 shrink-0 text-dimmed"
            />
          </component>
        </div>
      </FormSection>

      <FormSection
        v-if="toy.bhaptics"
        title="On your gear"
        :description="fill === 'depth' ? 'Rows light from the bottom up as the level grows, like a socket\'s depth.' : 'Every motor here plays at the level.'"
      >
        <div class="flex flex-wrap items-center gap-4">
          <BhapticsMotors
            :position="toy.bhaptics.position"
            :levels="bhapticsState.playing[toy.bhaptics.position]"
          />
          <SegmentedControl
            :model-value="fill"
            :items="[
              { value: 'all', label: 'All at once' },
              { value: 'depth', label: 'From the bottom up' }
            ]"
            aria-label="How it lights"
            @update:model-value="setFill"
          />
        </div>
      </FormSection>

      <FormSection
        v-if="toy.owo"
        title="On your suit"
        description="Both muscles fire at the level, under each one's strength in Settings > OWO. Below the dead zone nothing plays."
      >
        <div class="flex flex-wrap gap-4">
          <OwoMuscles
            side="front"
            :levels="owoState.playing"
          />
          <OwoMuscles
            side="back"
            :levels="owoState.playing"
          />
        </div>
      </FormSection>

      <FormSection
        v-if="shown"
        title="Max strength"
        :description="toy.bhaptics || toy.owo ? 'These never go past it from haptics, whatever its sources say.' : 'This toy never goes past it from haptics, whatever its sources say.'"
      >
        <div class="grid gap-2">
          <div class="flex justify-end text-sm">
            <b class="font-semibold text-highlighted tabular-nums">{{ pct(shown.maxStrength) }}</b>
          </div>
          <USlider
            :model-value="Math.round(shown.maxStrength * 100)"
            :min="0"
            :max="100"
            :step="1"
            size="lg"
            :disabled="inherited"
            aria-label="Max strength"
            @update:model-value="setMaxStrength"
          />
        </div>
      </FormSection>
    </div>

    <EmptyState
      v-else
      icon="i-lucide-hand-heart"
      title="Nothing to drive yet"
      description="Connect a toy in Intiface Central or your gear in bHaptics Player, then pick which part of your avatar it follows."
    />

    <template
      v-if="toy && editedSource"
      #footer
    >
      <UButton
        icon="i-lucide-check"
        :to="`/haptics/${toy.key}`"
      >
        Done
      </UButton>
    </template>
  </PageLayout>
</template>
