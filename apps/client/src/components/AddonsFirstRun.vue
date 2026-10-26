<script setup lang="ts">
import AssistantsSettings from '@renderer/components/settings/AssistantsSettings.vue'
import BhapticsSettings from '@renderer/components/settings/BhapticsSettings.vue'
import BoardsSettings from '@renderer/components/settings/BoardsSettings.vue'
import IntifaceSettings from '@renderer/components/settings/IntifaceSettings.vue'
import OpenShockSettings from '@renderer/components/settings/OpenShockSettings.vue'
import OverlaySettings from '@renderer/components/settings/OverlaySettings.vue'
import OwoSettings from '@renderer/components/settings/OwoSettings.vue'
import FormSection from '@renderer/components/ui/FormSection.vue'
import type { Addon } from '@renderer/composables/useAddons'
import { useAddons } from '@renderer/composables/useAddons'
import type { Component } from 'vue'
import { computed, ref } from 'vue'

// A new install asks once which add-ons you use, as big tiles you tap to turn on. Then one step per
// add-on that needs something from you (its Settings section, needs first), and a summary. Every
// step can wait: Set up later keeps it on, and Settings › Add-ons shows it as Needs setup. Quitting
// halfway comes back to the first step not answered yet. Skip leaves everything off.
const { addons, getAddon, setEnabled, setSetup, finishSetup } = useAddons()
const toast = useToast()

// The step of each add-on: its Settings section. Avatar haptics has no section, only a note.
// The leash works as it is, so it has no step.
const STEPS: Record<string, Component | null> = {
  openshock: OpenShockSettings,
  intiface: IntifaceSettings,
  boards: BoardsSettings,
  haptics: null,
  bhaptics: BhapticsSettings,
  owo: OwoSettings,
  overlay: OverlaySettings,
  mcp: AssistantsSettings
}

const enabledIds = (): Set<string> => new Set(addons.value.filter((addon) => addon.enabled).map((addon) => addon.id))

const picked = ref(enabledIds())
const saving = ref(false)

const toggle = (addon: Addon): void => {
  const next = new Set(picked.value)

  if (next.has(addon.id)) next.delete(addon.id)
  else next.add(addon.id)
  picked.value = next
}

// These add-ons and whatever they need, needs first.
const withNeeds = (ids: Set<string>): string[] => {
  const order: string[] = []
  const add = (id: string): void => {
    if (order.includes(id)) return
    const addon = getAddon(id)

    addon?.needs.forEach(add)
    // Needs one of several: the first, unless one of them is already picked.
    if (addon?.needsAny.length && !addon.needsAny.some((need) => ids.has(need) || order.includes(need))) add(addon.needsAny[0]!)
    order.push(id)
  }

  addons.value.filter((addon) => ids.has(addon.id)).forEach((addon) => add(addon.id))

  return order
}

const toTurnOn = computed(() => withNeeds(picked.value))
// The steps of what's on, needs first.
const steps = computed(() => withNeeds(enabledIds()).filter((id) => id in STEPS).map((id) => getAddon(id)!))

// 'pick', an add-on id, or 'summary'. Coming back after quitting halfway: the first step not answered.
const stage = ref(picked.value.size ? (steps.value.find((addon) => !addon.setup)?.id ?? 'summary') : 'pick')
const stages = computed(() => ['pick', ...steps.value.map((addon) => addon.id), 'summary'])
const current = computed(() => getAddon(stage.value))
const go = (offset: number): void => {
  stage.value = stages.value[stages.value.indexOf(stage.value) + offset] ?? 'summary'
}

// Shockers and the OWO suit wait for a tick before Continue, the first time.
const confirmed = ref(new Set<string>())
const isSafety = (addon: Addon): boolean => addon.badges.includes('shocks') || addon.badges.includes('electrical')
const blocked = computed(() => !!current.value && isSafety(current.value) && current.value.setup !== 'done' && !confirmed.value.has(current.value.id))
const toggleConfirmed = (id: string, value: boolean): void => {
  const next = new Set(confirmed.value)

  if (value) next.add(id)
  else next.delete(id)
  confirmed.value = next
}

const run = async (action: () => Promise<void>): Promise<void> => {
  saving.value = true
  try {
    await action()
  } catch (error) {
    toast.add({ title: 'Could not save your add-ons', description: String(error), color: 'error' })
  } finally {
    saving.value = false
  }
}

// Turns on the picks (and what they need), turns off what was unticked after going back.
const applyPicks = (): Promise<void> =>
  run(async () => {
    const turnOn = toTurnOn.value

    for (const id of turnOn) await setEnabled(id, true)
    for (const id of [...enabledIds()].filter((id) => !turnOn.includes(id)).reverse()) await setEnabled(id, false)
    if (!turnOn.length) await finishSetup(false)
    // From the picks: what's on reaches us a moment later, through addons-update.
    else stage.value = turnOn.find((id) => id in STEPS) ?? 'summary'
  })

const answer = (setup: 'done' | 'later'): Promise<void> =>
  run(async () => {
    await setSetup(stage.value, setup)
    go(1)
  })

const skipRest = (): Promise<void> =>
  run(async () => {
    const from = steps.value.findIndex((addon) => addon.id === stage.value)

    for (const addon of steps.value.slice(from)) if (!addon.setup) await setSetup(addon.id, 'later')
    stage.value = 'summary'
  })

const skipAll = (): Promise<void> => run(() => finishSetup(false))
const finish = (): Promise<void> => run(() => finishSetup(false))

const summary = computed(() =>
  withNeeds(enabledIds()).map((id) => {
    const addon = getAddon(id)!
    const result =
      addon.setup === 'later'
        ? { label: 'Set up later', color: 'neutral' as const }
        : addon.status.state === 'ok'
          ? { label: 'Ready', color: 'success' as const }
          : { label: 'Needs attention', color: 'warning' as const }

    return { addon, ...result, fix: id in STEPS && result.label !== 'Ready' }
  })
)

const DOT: Record<string, string> = { ok: 'bg-success', busy: 'bg-warning animate-pulse', warn: 'bg-warning', error: 'bg-error', off: 'bg-(--ui-text-dimmed)' }

const railMark = (id: string, index: number): string => {
  const setup = getAddon(id)?.setup

  return setup === 'done' ? '✓' : setup === 'later' ? '–' : String(index + 1)
}
</script>

<template>
  <!-- What do you use? -->
  <div
    v-if="stage === 'pick'"
    class="grid min-h-0 overflow-auto p-6"
  >
    <div class="m-auto grid w-full max-w-3xl gap-5">
      <div class="grid gap-1.5">
        <h1 class="text-3xl font-semibold text-highlighted">
          What do you use?
        </h1>
        <p class="text-muted">
          Turn on what you own. The rest stays off: no connections, no extra pages. You can change this any time in Settings › Add-ons.
        </p>
      </div>

      <div class="grid grid-cols-2 gap-3 max-md:grid-cols-1">
        <button
          v-for="addon in addons"
          :key="addon.id"
          type="button"
          class="grid min-h-32 cursor-pointer content-start gap-2 rounded-panel border p-5 text-left transition-colors"
          :class="picked.has(addon.id) ? 'border-primary/60 bg-primary/14' : 'border-transparent glass hover:bg-elevated/60'"
          :aria-pressed="picked.has(addon.id)"
          @click="toggle(addon)"
        >
          <span class="flex items-center justify-between gap-3">
            <b class="text-lg font-semibold text-highlighted">{{ addon.name }}</b>
            <UIcon
              :name="picked.has(addon.id) ? 'i-lucide-circle-check' : 'i-lucide-circle'"
              class="size-6"
              :class="picked.has(addon.id) ? 'text-primary' : 'text-muted'"
            />
          </span>
          <span class="text-sm text-muted">{{ addon.pitch }}</span>
          <UBadge
            v-if="addon.badges.includes('shocks') || addon.badges.includes('electrical')"
            :label="addon.badges.includes('shocks') ? 'Shocks' : 'Electrical'"
            color="error"
            variant="subtle"
            class="w-fit rounded-full"
          />
        </button>
      </div>

      <div class="flex flex-wrap justify-end gap-2.5">
        <UButton
          color="neutral"
          variant="subtle"
          size="xl"
          class="rounded-2xl"
          :disabled="saving"
          @click="skipAll"
        >
          Skip
        </UButton>
        <UButton
          size="xl"
          class="rounded-2xl"
          icon="i-lucide-check"
          :loading="saving"
          @click="applyPicks"
        >
          {{ picked.size ? `Turn on ${picked.size} and continue` : 'Continue' }}
        </UButton>
      </div>
    </div>
  </div>

  <!-- One step per add-on, then the summary. -->
  <div
    v-else
    class="flex min-h-0"
  >
    <nav
      aria-label="Setup steps"
      class="flex w-64 shrink-0 flex-col gap-1 overflow-y-auto border-r border-(--ui-border) p-3 max-md:hidden"
    >
      <span class="px-2.5 pt-1 pb-2 font-mono text-[11.5px] tracking-widest text-muted uppercase">Setup</span>
      <button
        v-for="(id, index) in stages"
        :key="id"
        type="button"
        class="grid min-h-12 cursor-pointer grid-cols-[auto_minmax(0,1fr)] items-center gap-2.5 rounded-2xl border px-2.5 text-left transition-colors"
        :class="id === stage ? 'border-primary/50 bg-primary/14' : 'border-transparent hover:bg-(--aurora-glass)'"
        :aria-current="id === stage ? 'step' : undefined"
        @click="stage = id"
      >
        <span
          class="grid size-7 place-items-center rounded-full font-mono text-xs font-semibold"
          :class="getAddon(id)?.setup === 'done' ? 'bg-success/15 text-success' : getAddon(id)?.setup === 'later' ? 'bg-warning/15 text-warning' : 'bg-(--aurora-well) text-muted'"
        >{{ railMark(id, index) }}</span>
        <b class="truncate text-[14.5px] font-medium text-highlighted">{{ id === 'pick' ? 'What do you use?' : id === 'summary' ? 'Summary' : getAddon(id)?.name }}</b>
      </button>
    </nav>

    <div class="flex min-w-0 flex-1 flex-col">
      <div class="min-h-0 flex-1 overflow-auto p-6">
        <div class="mx-auto grid w-full max-w-3xl gap-5">
          <template v-if="current">
            <div class="grid gap-1.5">
              <span class="font-mono text-xs text-muted">Add-on {{ stages.indexOf(stage) }} of {{ steps.length }}</span>
              <h1 class="text-3xl font-semibold text-highlighted">
                Set up {{ current.name }}
              </h1>
              <div class="flex items-center gap-2 text-sm text-muted">
                <span
                  class="size-2.5 shrink-0 rounded-full"
                  :class="DOT[current.status.state]"
                />
                <span :class="current.status.state === 'error' ? 'text-error' : undefined">{{ current.status.text }}</span>
              </div>
            </div>

            <FormSection v-if="stage === 'haptics'">
              <p class="text-sm text-muted">
                Avatar haptics picks which toy each part of your avatar drives. It needs to see your avatar, so do this on the Haptics page while
                you're in VRChat. ToolKitOSC uses sensible defaults until then.
              </p>
            </FormSection>
            <component
              :is="STEPS[stage]"
              v-else
            />

            <UCheckbox
              v-if="isSafety(current) && current.setup !== 'done'"
              :model-value="confirmed.has(current.id)"
              :label="current.badges.includes('shocks')
                ? 'I understand viewers can shock me through the controls I share, up to the limits set on each control.'
                : 'I\'ve read OWO\'s safety instructions and calibrated my limits in the OWO app.'"
              @update:model-value="toggleConfirmed(current.id, !!$event)"
            />
          </template>

          <template v-else>
            <div class="grid gap-1.5">
              <h1 class="text-3xl font-semibold text-highlighted">
                You're set
              </h1>
              <p class="text-muted">
                Here's where each add-on stands. Anything left for later is in Settings › Add-ons.
              </p>
            </div>
            <div class="grid gap-2">
              <div
                v-for="row in summary"
                :key="row.addon.id"
                class="flex items-center gap-3 rounded-panel glass p-4"
              >
                <div class="grid min-w-0 flex-1 gap-0.5">
                  <b class="font-semibold text-highlighted">{{ row.addon.name }}</b>
                  <small class="truncate text-xs text-muted">{{ row.addon.status.text }}</small>
                </div>
                <UBadge
                  :label="row.label"
                  :color="row.color"
                  variant="subtle"
                  class="rounded-full"
                />
                <UButton
                  v-if="row.fix"
                  color="neutral"
                  variant="subtle"
                  class="rounded-2xl"
                  @click="stage = row.addon.id"
                >
                  Fix
                </UButton>
              </div>
            </div>
          </template>
        </div>
      </div>

      <div class="flex flex-wrap items-center justify-between gap-2.5 border-t border-(--ui-border) px-6 py-4">
        <UButton
          color="neutral"
          variant="ghost"
          size="xl"
          class="rounded-2xl"
          :disabled="saving"
          @click="go(-1)"
        >
          Back
        </UButton>
        <div class="flex flex-wrap gap-2.5">
          <template v-if="current">
            <UButton
              color="neutral"
              variant="ghost"
              size="xl"
              class="rounded-2xl"
              :disabled="saving"
              @click="skipRest"
            >
              Skip the rest
            </UButton>
            <UButton
              color="neutral"
              variant="subtle"
              size="xl"
              class="rounded-2xl"
              :disabled="saving"
              @click="answer('later')"
            >
              Set up later
            </UButton>
            <UButton
              size="xl"
              class="rounded-2xl"
              :loading="saving"
              :disabled="blocked"
              @click="answer('done')"
            >
              Continue
            </UButton>
          </template>
          <UButton
            v-else
            size="xl"
            class="rounded-2xl"
            icon="i-lucide-check"
            :loading="saving"
            @click="finish"
          >
            Open ToolKitOSC
          </UButton>
        </div>
      </div>
    </div>
  </div>
</template>
