<script setup lang="ts">
import ChoiceCard from '@renderer/components/ui/ChoiceCard.vue'
import EmptyState from '@renderer/components/ui/EmptyState.vue'
import FormSection from '@renderer/components/ui/FormSection.vue'
import { useAiAccess } from '@renderer/composables/useAiAccess'
import { useAiGenerationStatus } from '@renderer/composables/useAiGenerationStatus'
import { useAiSession } from '@renderer/composables/useAiSession'
import { useAvatarDetails } from '@renderer/composables/useAvatarDetails'
import { useControls } from '@renderer/composables/useControls'
import { usePresets } from '@renderer/composables/usePresets'
import { timeAgo } from '@renderer/utils/time'
import { CONTROL_TYPE_COLORS, CONTROL_TYPE_LABELS } from '@toolkitosc/shared-ui'
import { computed, onMounted, ref, toRaw, watch } from 'vue'
import { useRouter } from 'vue-router'

// AI control suggestions as a page instead of a modal: pick a profile (and a model, if an admin
// allowed it), generate, then review the proposed groups and choose which to add - the old modal
// could only add all or nothing. Nothing changes in your controls until you add them. This
// session's runs stay in the sidebar with their full result: reopen one (on the avatar it was made
// for) to add more of its groups.
const PROFILE_ICONS: Record<string, string> = { light: 'i-lucide-zap', balanced: 'i-lucide-gauge', heavy: 'i-lucide-cpu' }

const router = useRouter()
const toast = useToast()
const { access } = useAiAccess()
const { avatarDetails } = useAvatarDetails()
const { includedParameters } = usePresets()
const { controls, setGroups } = useControls()
const { statusMessage } = useAiGenerationStatus()
const { options, optionsError, optionsLoading, result, openRun, openRunById, markAdded, runError, generating, runs, loadOptions, run } = useAiSession()

onMounted(() => void loadOptions())

const profile = ref('balanced')
const provider = ref<string>()

watch(
  options,
  (value) => {
    if (value && !provider.value) provider.value = value.providers.find((p) => p.configured)?.id
  },
  { immediate: true }
)

const selectedProfile = computed(() => options.value?.profiles.find((p) => p.id === profile.value))
const remaining = computed(() => options.value?.remainingCredits)
const runsLeft = computed(() => (remaining.value !== undefined && selectedProfile.value ? Math.floor(remaining.value / selectedProfile.value.cost) : undefined))
const canSelectModel = computed(() => access.value.canSelectModel)

const canGenerate = computed(
  () => !!avatarDetails.value && !generating.value && !!selectedProfile.value && (runsLeft.value ?? 0) > 0 && (!canSelectModel.value || !!provider.value)
)

const generate = (): void => {
  if (!canGenerate.value || !selectedProfile.value) return

  picked.value = new Set()
  void run(canSelectModel.value ? provider.value : undefined, profile.value, selectedProfile.value.label)
}

// Which proposed groups to add; every group not added yet starts selected.
const picked = ref<Set<number>>(new Set())

watch(
  () => openRun.value?.id,
  () => (picked.value = new Set(result.value?.map((_, index) => index).filter((index) => !openRun.value?.added.has(index)) ?? [])),
  { immediate: true }
)

const togglePicked = (index: number): void => {
  const next = new Set(picked.value)

  if (next.has(index)) next.delete(index)
  else next.add(index)

  picked.value = next
}

const isAdded = (index: number): boolean => !!openRun.value?.added.has(index)

const pickedGroups = computed(() => result.value?.filter((_, index) => picked.value.has(index)) ?? [])
const pickedControls = computed(() => pickedGroups.value.reduce((n, group) => n + group.controls.length, 0))

// Appended to the existing groups, never replacing them. Copies with fresh ids, so a group added
// twice from the same run (or from two runs) never shares ids with another.
const addPicked = (): void => {
  if (!pickedGroups.value.length) return

  const count = pickedGroups.value.length
  const copies = pickedGroups.value.map((group) => ({
    ...structuredClone(toRaw(group)),
    id: crypto.randomUUID(),
    controls: group.controls.map((control) => ({ ...structuredClone(toRaw(control)), id: crypto.randomUUID() }))
  }))

  markAdded([...picked.value])
  setGroups([...controls.value, ...copies])
  toast.add({ title: `Added ${count} ${count === 1 ? 'group' : 'groups'} (${pickedControls.value} controls)`, icon: 'i-lucide-sparkles', color: 'success' })
  result.value = null
  void router.push('/controls')
}
</script>

<template>
  <PageLayout>
    <template #sidebar>
      <div class="flex min-h-9 shrink-0 items-center">
        <b class="text-xl font-semibold text-highlighted">AI suggestions</b>
      </div>

      <div
        v-if="options"
        class="grid shrink-0 gap-2 rounded-field glass p-3.5"
      >
        <div class="flex items-baseline gap-1.5">
          <b class="text-2xl font-semibold text-highlighted tabular-nums">{{ options.remainingCredits }}</b>
          <span class="text-sm text-muted">of {{ options.dailyCredits }} credits</span>
        </div>
        <UProgress
          :model-value="options.remainingCredits"
          :max="options.dailyCredits"
          size="md"
        />
        <span class="text-xs text-muted">Left today · resets at midnight UTC</span>
      </div>

      <div class="px-1 pt-1 font-mono text-[10.5px] tracking-widest text-muted uppercase">
        This session
      </div>
      <div class="-mr-1.5 flex min-h-0 flex-1 flex-col gap-1.5 overflow-y-auto pr-1.5">
        <!-- A run reopens on the avatar it was made for; runs for another avatar stay listed but
        can't open, since their groups map to that avatar's parameters. -->
        <button
          v-for="entry in runs"
          :key="entry.id"
          type="button"
          class="grid min-h-14 cursor-pointer grid-cols-[auto_minmax(0,1fr)] items-center gap-x-2.5 rounded-2xl border px-2.5 py-1.5 text-left transition-colors disabled:cursor-not-allowed disabled:opacity-50"
          :class="entry.id === openRun?.id ? 'border-primary/50 bg-primary/14' : 'border-transparent enabled:hover:bg-(--aurora-glass)'"
          :disabled="entry.avatarId !== avatarDetails?.id || generating"
          :title="entry.avatarId !== avatarDetails?.id ? `Made for ${entry.avatarName} - load that avatar to open it` : undefined"
          @click="openRunById(entry.id)"
        >
          <span class="row-span-2 grid size-8.5 place-items-center rounded-xl bg-(--aurora-well) text-primary">
            <UIcon
              :name="entry.added.size ? 'i-lucide-folder-check' : 'i-lucide-sparkles'"
              class="size-4.5"
            />
          </span>
          <b class="truncate text-sm font-semibold text-highlighted">{{ entry.profile }} · {{ entry.groups.length }} {{ entry.groups.length === 1 ? 'group' : 'groups' }}</b>
          <small class="col-start-2 truncate text-xs text-muted">
            <template v-if="entry.avatarId !== avatarDetails?.id">{{ entry.avatarName }} · {{ timeAgo(entry.at) }}</template>
            <template v-else>{{ entry.controls }} controls{{ entry.added.size ? ` · ${entry.added.size} added` : '' }} · {{ timeAgo(entry.at) }}</template>
          </small>
        </button>
        <p
          v-if="!runs.length"
          class="px-1 text-sm text-muted"
        >
          No runs yet.
        </p>
      </div>
    </template>

    <template #header>
      <PageHeader
        title="Suggest controls with AI"
        :meta="avatarDetails ? `Proposes groups from ${includedParameters.length} parameters. Nothing changes until you add them.` : undefined"
      />
    </template>

    <EmptyState
      v-if="!access.hasAccess"
      icon="i-lucide-sparkles"
      title="AI suggestions aren't enabled"
      description="An admin of this server needs to give your account access."
    />
    <EmptyState
      v-else-if="!avatarDetails"
      icon="i-lucide-radio-tower"
      title="VRChat not detected"
      description="AI suggestions work from your avatar's parameters. Load into VRChat with OSC on."
    />

    <div
      v-else
      class="grid max-w-4xl grid-cols-1 gap-3.5"
    >
      <UAlert
        v-if="optionsError"
        color="error"
        variant="subtle"
        icon="i-lucide-triangle-alert"
        title="Couldn't load AI options"
        :description="optionsError"
        :actions="[{ label: 'Try again', onClick: () => void loadOptions() }]"
      />

      <FormSection title="Profile">
        <div
          v-if="optionsLoading && !options"
          class="grid gap-2.5 sm:grid-cols-3"
        >
          <USkeleton
            v-for="n in 3"
            :key="n"
            class="h-21.5 rounded-field"
          />
        </div>
        <div
          v-else
          class="grid gap-2.5 sm:grid-cols-3"
        >
          <ChoiceCard
            v-for="item in options?.profiles ?? []"
            :key="item.id"
            :title="item.label"
            :description="item.description"
            :icon="PROFILE_ICONS[item.id] ?? 'i-lucide-sparkles'"
            :selected="profile === item.id"
            :disabled="generating"
            @click="profile = item.id"
          >
            <template #aside>
              <span class="rounded-full bg-secondary/15 px-2 py-0.5 text-xs font-medium text-secondary">{{ item.cost }} cr</span>
            </template>
          </ChoiceCard>
        </div>

        <UFormField
          v-if="canSelectModel && options"
          label="Model"
          description="You can pick a model because an admin allowed it on your account."
        >
          <div class="flex flex-wrap gap-2">
            <UButton
              v-for="item in options.providers"
              :key="item.id"
              size="md"
              class="rounded-full"
              :color="provider === item.id ? 'primary' : 'neutral'"
              :variant="provider === item.id ? 'soft' : 'subtle'"
              :disabled="!item.configured || generating"
              @click="provider = item.id"
            >
              {{ item.label }}{{ item.configured ? '' : ' · not configured' }}
            </UButton>
          </div>
        </UFormField>
      </FormSection>

      <UButton
        v-if="!generating && !result"
        size="xl"
        icon="i-lucide-sparkles"
        block
        class="h-15 text-base"
        :disabled="!canGenerate"
        @click="generate"
      >
        <template v-if="(runsLeft ?? 0) > 0">
          Generate · {{ selectedProfile?.cost }} credits (≈{{ runsLeft }} more today)
        </template>
        <template v-else>
          Not enough credits left today
        </template>
      </UButton>

      <FormSection
        v-if="generating"
        class="justify-items-center text-center"
      >
        <UIcon
          name="i-lucide-sparkles"
          class="size-9 animate-pulse text-primary"
        />
        <h2 class="text-lg font-semibold text-highlighted">
          {{ statusMessage ?? 'Thinking…' }}
        </h2>
        <UProgress
          animation="carousel"
          class="max-w-sm"
        />
        <p class="text-[13px] text-muted">
          This can take a few minutes. You can leave this page; the result waits here.
        </p>
      </FormSection>

      <UAlert
        v-if="runError"
        color="error"
        variant="subtle"
        icon="i-lucide-triangle-alert"
        title="Generation failed"
        :description="runError"
      />

      <template v-if="result">
        <div class="flex flex-wrap items-center gap-2.5">
          <h2 class="flex-1 text-xl font-semibold text-highlighted">
            {{ result.length }} groups, {{ result.reduce((n, g) => n + g.controls.length, 0) }} controls proposed
          </h2>
          <UButton
            icon="i-lucide-x"
            color="neutral"
            variant="ghost"
            @click="result = null"
          >
            Close
          </UButton>
        </div>

        <div class="grid gap-3 sm:grid-cols-2">
          <ChoiceCard
            v-for="(group, index) in result"
            :key="group.id"
            :title="group.name"
            :description="isAdded(index) ? `${group.controls.length} controls · already added` : `${group.controls.length} controls`"
            :selected="picked.has(index)"
            :icon="isAdded(index) ? 'i-lucide-folder-check' : 'i-lucide-folder'"
            @click="togglePicked(index)"
          >
            <div class="mt-1 flex flex-wrap gap-1.5">
              <UBadge
                v-for="control in group.controls"
                :key="control.id"
                :color="CONTROL_TYPE_COLORS[control.type]"
                variant="subtle"
              >
                {{ control.name }} · {{ CONTROL_TYPE_LABELS[control.type] }}
              </UBadge>
            </div>
          </ChoiceCard>
        </div>

        <UButton
          size="xl"
          icon="i-lucide-plus"
          block
          class="h-15 text-base"
          :disabled="!pickedGroups.length"
          @click="addPicked"
        >
          Add {{ pickedGroups.length }} {{ pickedGroups.length === 1 ? 'group' : 'groups' }} ({{ pickedControls }} controls)
        </UButton>
      </template>
    </div>
  </PageLayout>
</template>
