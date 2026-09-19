<script setup lang="ts">
import type { AiAvatarLimitInfo, AiProfileInfo, AiProviderInfo } from '@renderer/composables/useAiControlSuggestions'
import { useAiControlSuggestions } from '@renderer/composables/useAiControlSuggestions'
import { useAiCredits } from '@renderer/composables/useAiCredits'
import { useAvatarDetails } from '@renderer/composables/useAvatarDetails'
import { useControls } from '@renderer/composables/useControls'
import type { ControlGroup } from '@vrc-osc-toolkit/shared-ui'
import { CONTROL_TYPE_COLORS, CONTROL_TYPE_LABELS } from '@vrc-osc-toolkit/shared-ui'
import { computed, ref, watch } from 'vue'

const open = defineModel<boolean>('open')

const { loading, error, listOptions, generate } = useAiControlSuggestions()
const { controls, setGroups } = useControls()
const { avatarDetails } = useAvatarDetails()
const { latestByProvider } = useAiCredits()
const toast = useToast()

const providers = ref<AiProviderInfo[]>([])
const profiles = ref<AiProfileInfo[]>([])
const dailyCredits = ref(0)
const avatarLimit = ref<AiAvatarLimitInfo | null>(null)
const optionsLoading = ref(true)
const optionsError = ref<string | null>(null)
const selectedProvider = ref<string>()
// 'balanced' is the sensible middle default - matches profiles.ts's own default tier.
const selectedProfile = ref<string>('balanced')
const result = ref<ControlGroup[] | null>(null)

const loadOptions = async (): Promise<void> => {
  optionsLoading.value = true

  try {
    const options = await listOptions()

    providers.value = options.providers
    profiles.value = options.profiles
    dailyCredits.value = options.dailyCredits
    avatarLimit.value = options.avatarLimit
    if (!selectedProvider.value) selectedProvider.value = providers.value.find((p) => p.configured)?.id
    optionsError.value = null
  } catch (err) {
    // Previously swallowed silently, which looked identical to "no providers configured" - an
    // empty list with no clue why (e.g. the CORS preflight this route didn't use to answer, or
    // the desktop client not being logged in). Surfaced the same way generate()'s own failure is.
    optionsError.value = err instanceof Error ? err.message : String(err)
  } finally {
    optionsLoading.value = false
  }
}

// `immediate: true` matters here, not just style: whether useOverlay() keeps this component
// mounted across opens (so `open` flips false->true on a later re-open) or remounts it fresh each
// time (so `open` is already true at setup), a plain non-immediate watch only catches the first
// case - it never fires at all on a value that's already true when the watcher is created, which
// left the modal stuck on its loading spinner forever the one time this ran without `immediate`.
// Covers both cases and keeps credits/avatar-remaining from going stale either way; handleGenerate
// below refreshes them again afterward too, since a generation (successful or not - the
// avatar-limit check still consumes even when the credit check that runs after it then fails, see
// the server's suggest-controls.post.ts) always changes at least one of them.
watch(
  open,
  (isOpen) => {
    if (isOpen) void loadOptions()
  },
  { immediate: true }
)

// Applies a server-pushed credit/avatar-limit update (see useWebsocketHost.ts's
// 'ai-credits-update' handler) the instant it arrives, for whichever provider it's for - not
// gated to the currently-selected one, so switching providers later still shows a number that was
// kept fresh in the background. Only a fallback of last resort now: handleGenerate below still
// refetches via REST too, for whenever this client has no live host WS connection to push over.
watch(
  latestByProvider,
  (updates) => {
    for (const update of Object.values(updates)) {
      const provider = providers.value.find((p) => p.id === update.provider)

      if (provider) provider.remainingCredits = update.remainingCredits
      dailyCredits.value = update.dailyCredits

      // avatarLimit is per-avatar - only applied when it matches whatever avatar is currently
      // loaded, so switching avatars between request and push doesn't show a stale-avatar number.
      if (update.avatarId === avatarDetails.value?.id) {
        avatarLimit.value = { max: update.avatarLimit.max, remaining: update.avatarLimit.remaining }
      }
    }
  },
  { deep: true }
)

// Credits left today in the selected provider's shared pool - stable across profile switches,
// only the selected profile's own cost (below) changes what a generation would spend from it.
const remainingCredits = computed<number | undefined>(
  () => providers.value.find((p) => p.id === selectedProvider.value)?.remainingCredits
)

const selectedCost = computed<number | undefined>(
  () => profiles.value.find((p) => p.id === selectedProfile.value)?.cost
)

// How many more generations at the currently selected profile's cost the remaining balance
// actually buys - a convenience derived from the two numbers above, not tracked separately.
const estimatedRunsLeft = computed<number | undefined>(() => {
  if (remainingCredits.value === undefined || !selectedCost.value) return undefined

  return Math.floor(remainingCredits.value / selectedCost.value)
})

const handleGenerate = async (): Promise<void> => {
  if (!selectedProvider.value || !selectedProfile.value) return

  result.value = null

  try {
    result.value = await generate(selectedProvider.value, selectedProfile.value)
  } catch {
    // error.value is already set by generate() - surfaced in the template below.
  } finally {
    // Refreshes credits/avatar-remaining regardless of outcome - even a failed attempt can have
    // spent the avatar-limit check (see loadOptions's comment above).
    void loadOptions()
  }
}

const totalControls = (groups: ControlGroup[]): number =>
  groups.reduce((sum, g) => sum + g.controls.length, 0)

// Appends rather than replaces - never destroys whatever control groups already exist for this
// avatar, matching how a preset/control is normally added one at a time.
const handleApply = (): void => {
  if (!result.value) return

  setGroups([...controls.value, ...result.value])
  toast.add({
    title: `Added ${result.value.length} group(s) / ${totalControls(result.value)} control(s)`,
    icon: 'i-lucide-check',
    color: 'success'
  })
  result.value = null
  open.value = false
}
</script>

<template>
  <UModal
    v-model:open="open"
    :ui="{ content: 'max-w-4xl', body: 'flex flex-col gap-4' }"
    title="AI: Suggest Controls"
    description="Groups and controls are proposed from this avatar's current parameters (filtered the same way presets are) - nothing is applied until you review and confirm."
  >
    <template #body>
      <div class="flex flex-wrap items-center gap-2">
        <USelect
          v-model="selectedProvider"
          :items="
            providers.map((p) => ({
              label: p.configured ? p.label : `${p.label} (not configured)`,
              value: p.id,
              disabled: !p.configured
            }))
          "
          :loading="optionsLoading"
          placeholder="Provider"
          class="min-w-40"
        />
        <USelect
          v-model="selectedProfile"
          :items="profiles.map((p) => ({ label: `${p.label} (${p.cost} cr)`, value: p.id }))"
          :loading="optionsLoading"
          placeholder="Profile"
          class="min-w-40"
        />
        <button
          type="button"
          class="ai-generate-btn"
          :class="{ 'is-generating': loading }"
          :disabled="!selectedProvider || !selectedProfile || loading"
          @click="handleGenerate"
        >
          <UIcon
            name="i-lucide-sparkles"
            class="size-4"
            :class="{ 'animate-spin': loading }"
          />
          {{ loading ? 'Generating…' : 'Generate' }}
        </button>
      </div>

      <p
        v-if="remainingCredits !== undefined"
        class="text-xs text-muted"
      >
        {{ remainingCredits }} / {{ dailyCredits }} credits left today for this provider
        <template v-if="estimatedRunsLeft !== undefined">
          - ≈{{ estimatedRunsLeft }} more at this profile's cost
        </template>
      </p>

      <!-- Surfaced up front, not just as a 429 message after the fact - this limit is per avatar
      and separate from the credit pool above, so it can be hit even with plenty of credits left. -->
      <p
        v-if="avatarLimit"
        class="text-xs text-muted"
      >
        Max {{ avatarLimit.max }} generations/day for this avatar
        <template v-if="avatarLimit.remaining !== null">
          ({{ avatarLimit.remaining }} left today)
        </template>
      </p>

      <p
        v-if="selectedProfile"
        class="text-xs text-muted"
      >
        {{ profiles.find((p) => p.id === selectedProfile)?.description }}
      </p>

      <UAlert
        v-if="optionsError"
        color="error"
        variant="subtle"
        :title="`Couldn't load providers: ${optionsError}`"
      />

      <UAlert
        v-if="error"
        color="error"
        variant="subtle"
        :title="error"
      />

      <template v-if="result">
        <p class="text-sm text-muted">
          {{ result.length }} group(s), {{ totalControls(result) }} control(s) proposed.
        </p>

        <UScrollArea
          :items="result"
          class="h-96"
          :ui="{ item: 'pb-3 last:pb-0' }"
        >
          <template #default="{ item: group }">
            <UCard :ui="{ body: 'p-3 sm:p-3' }">
              <p class="mb-2 font-medium">
                {{ group.name }}
              </p>
              <div class="flex flex-wrap gap-1.5">
                <UBadge
                  v-for="control in group.controls"
                  :key="control.id"
                  :color="CONTROL_TYPE_COLORS[control.type]"
                  variant="subtle"
                >
                  {{ control.name }}
                  <span class="ml-1 opacity-70">({{ CONTROL_TYPE_LABELS[control.type] }})</span>
                </UBadge>
              </div>
            </UCard>
          </template>
        </UScrollArea>

        <UButton
          color="primary"
          icon="i-lucide-plus"
          @click="handleApply"
        >
          Add to Controls
        </UButton>
      </template>
    </template>
  </UModal>
</template>

<style scoped>
/* Same themed gradient affordance as App.vue's trigger button beside "Add Group" - this is the
   one that actually plays while a request is in flight (`.is-generating`), a continuous shimmer
   rather than App.vue's hover-only sweep, since this button's whole job is signalling "working". */
.ai-generate-btn {
  position: relative;
  display: inline-flex;
  align-items: center;
  gap: 0.375rem;
  padding: 0.5rem 0.875rem;
  border-radius: calc(var(--ui-radius) * 2);
  font-size: 0.875rem;
  line-height: 1.25rem;
  font-weight: 500;
  color: white;
  background-image: linear-gradient(
    115deg,
    var(--ui-color-primary-500),
    var(--ui-color-secondary-500),
    var(--ui-color-primary-500)
  );
  background-size: 200% 100%;
  background-position: 0% 0%;
  transition:
    transform 0.15s ease,
    box-shadow 0.15s ease,
    background-position 0.6s ease,
    opacity 0.15s ease;
}

.ai-generate-btn:hover:not(:disabled):not(.is-generating) {
  background-position: 100% 0%;
  transform: translateY(-1px);
  box-shadow: 0 4px 14px color-mix(in oklab, var(--ui-color-primary-500) 45%, transparent);
}

.ai-generate-btn:disabled {
  opacity: 0.6;
  cursor: default;
  /* Disabled while generating (see :disabled="... || loading" above) - without this, hovering a
  disabled button can still visually register in some engines, and the base `transition` on
  background-position/transform/box-shadow would then race the shimmer keyframe animation below
  for the same properties, fighting/stalling it right as the mouse moves over the button. */
  pointer-events: none;
}

.ai-generate-btn.is-generating {
  /* Hands background-position over to the keyframe animation exclusively - left on, the base
  `transition` would keep trying to interpolate it back toward the idle position too, jittering
  against the animation every frame. */
  transition-property: opacity;
  animation: ai-generate-shimmer 1.8s linear infinite;
}

@keyframes ai-generate-shimmer {
  to {
    background-position: 200% 0%;
  }
}
</style>
