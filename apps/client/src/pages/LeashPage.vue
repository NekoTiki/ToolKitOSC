<script setup lang="ts">
import LeashCompass from '@renderer/components/leash/LeashCompass.vue'
import EmptyState from '@renderer/components/ui/EmptyState.vue'
import FormSection from '@renderer/components/ui/FormSection.vue'
import { useAreYouSureModal } from '@renderer/composables/useAreYouSureModal'
import { movesYou, useLeash } from '@renderer/composables/useLeash'
import type { LeashCheck, LeashProblem } from '@renderer/lib/tauri-bridge'
import { api } from '@renderer/lib/tauri-bridge'
import { computed, ref } from 'vue'

// Leash: someone grabs the leash on your avatar in VRChat and pulls, and you walk toward them.
// The live compass on the left, the worn avatar's leashes, settings and setup checks on the right.
const toast = useToast()
const { openModal: confirm } = useAreYouSureModal()
const { settings, state, avatarName, avatar, change, changeAvatar } = useLeash()

const pct = (value: number): string => `${Math.round(value * 100)} %`

const forward = computed(() => state.value.directions.zPos - state.value.directions.zNeg)
const right = computed(() => state.value.directions.xPos - state.value.directions.xNeg)

const motion = computed(() => {
  if (state.value.testing) return { label: 'Test pull', class: 'bg-secondary/15 text-secondary' }
  if (state.value.held.length && state.value.blocked === 'paused') return { label: 'Stop everything is on', class: 'bg-error/15 text-error' }
  if (state.value.held.length && state.value.blocked === 'off') return { label: 'Grabbed · leash off', class: 'bg-(--aurora-well) text-muted' }
  if (state.value.held.length && state.value.blocked === 'silent') return { label: 'Waiting for VRChat', class: 'bg-warning/15 text-warning' }

  switch (state.value.output.motion) {
    case 'running':
      return { label: 'Running', class: 'bg-warning/15 text-warning' }
    case 'walking':
      return { label: 'Walking', class: 'bg-secondary/15 text-secondary' }
    case 'grabbed':
      return { label: 'Grabbed', class: 'bg-primary/15 text-primary' }
    default:
      return { label: state.value.held.length ? 'Grabbed · not a leash' : 'Not held', class: 'bg-(--aurora-well) text-muted' }
  }
})

const toggleLeash = (leash: string): void =>
  changeAvatar((own) => {
    own.leashes[leash] = !movesYou(own, leash)
  })

const problemText = (problem: LeashProblem): string => {
  switch (problem.kind) {
    case 'no-leash':
      return 'Give your leash PhysBone a parameter name and add <Name>_IsGrabbed and <Name>_Stretch to your expression parameters.'
    case 'missing-stretch':
      return `Add ${problem.leash}_Stretch (Float) to your expression parameters.`
    case 'no-compass':
      return 'The compass is missing. Add it from the OSCLeash prefab.'
    case 'missing-directions':
      return `${problem.missing.join(', ')} ${problem.missing.length === 1 ? 'is' : 'are'} missing.`
    case 'wrong-type':
      return `${problem.parameter} is a ${problem.found}, it must be a ${problem.expected}.`
  }
}

interface CheckRow {
  key: string
  label: string
  check: LeashCheck
  detail: string
}

const checks = computed<CheckRow[]>(() => {
  const { detected, checks: live } = state.value
  const problems = detected.problems
  const leashProblems = problems.filter((p) => p.kind === 'no-leash' || p.kind === 'missing-stretch')
  const compassProblems = problems.filter((p) => p.kind === 'no-compass' || p.kind === 'missing-directions')
  const typeProblems = problems.filter((p) => p.kind === 'wrong-type')
  const rows: CheckRow[] = [
    {
      key: 'leash',
      label: 'Leash found',
      check: detected.leashes.length ? 'passed' : 'failed',
      detail: detected.leashes.length
        ? detected.leashes.map((leash) => `${leash}_IsGrabbed and ${leash}_Stretch`).join(' · ')
        : leashProblems.map(problemText).join(' ') || 'No leash found.'
    },
    {
      key: 'compass',
      label: 'Compass found',
      check: detected.compass ? 'passed' : 'failed',
      detail: detected.compass ? 'All six directions' : compassProblems.map(problemText).join(' ') || 'The compass is missing.'
    },
    ...typeProblems.map((problem, index) => ({ key: `type-${index}`, label: 'Wrong parameter type', check: 'failed' as const, detail: problemText(problem) }))
  ]

  if (!detected.leashes.length || !detected.compass) return rows

  const waiting = 'Waiting for a pull'

  rows.push(
    {
      key: 'stretches',
      label: 'Leash stretches',
      check: live.stretches,
      detail: live.stretches === 'failed' ? "Your leash can't stretch. Set Max stretch above 0 on its PhysBone." : live.stretches === 'passed' ? 'Pulling stretches it' : waiting
    },
    {
      key: 'sees',
      label: 'Compass sees the needle',
      check: live.compass,
      detail:
        live.compass === 'failed'
          ? "The compass doesn't see the needle. Check that its contacts allow self and the needle's sender is on."
          : live.compass === 'passed'
            ? 'The contacts react to a pull'
            : waiting
    },
    {
      key: 'needle',
      label: 'Needle follows the leash',
      check: live.needle,
      detail: live.needle === 'passed' ? 'The direction changes as you pull different ways' : `${waiting} in two directions. If it never changes, point the Aim Needle's source at the last leash bone.`
    }
  )

  return rows
})

const CHECK_ICON: Record<LeashCheck, { name: string; class: string }> = {
  passed: { name: 'i-lucide-circle-check', class: 'text-success' },
  failed: { name: 'i-lucide-circle-x', class: 'text-error' },
  waiting: { name: 'i-lucide-circle-dashed', class: 'text-muted' }
}

// Turning moves your view on its own, so turning it on asks first.
const setTurning = async (enabled: boolean): Promise<void> => {
  if (enabled) {
    const ok = await confirm({
      title: 'Turn toward the pull?',
      message: "Your view turns on its own while you're pulled, which can cause motion sickness. You can turn it off here at any time.",
      confirmText: 'Turn on',
      danger: false
    })

    if (!ok) return
  }

  changeAvatar((own) => (own.turning.enabled = enabled))
}

// Which way each leash hangs from you, from the end of its name (`Hang::of` in leash/engine.rs).
const HANGS = ['North', 'South', 'East', 'West']
const unnamedHang = computed(() =>
  state.value.detected.leashes.filter((leash) => movesYou(avatar.value, leash) && !HANGS.includes(leash.split('_').pop() ?? ''))
)

const testing = ref(false)

const testPull = async (): Promise<void> => {
  testing.value = true
  try {
    await api.leashTestPull()
  } catch (error) {
    toast.add({ title: 'Could not test the pull', description: String(error), color: 'error', icon: 'i-lucide-circle-x' })
  } finally {
    testing.value = false
  }
}
</script>

<template>
  <PageLayout>
    <template #header>
      <PageHeader
        title="Leash"
        :meta="state.avatarId ? avatarName : undefined"
      >
        <template
          v-if="settings"
          #actions
        >
          <USwitch
            :model-value="settings.enabled"
            label="Leash moves me"
            @update:model-value="change((target) => (target.enabled = $event))"
          />
        </template>
      </PageHeader>
    </template>

    <EmptyState
      v-if="!settings"
      icon="i-lucide-loader-circle"
      title="Loading…"
      class="[&>span:first-child]:animate-spin"
    />

    <EmptyState
      v-else-if="!state.avatarId"
      icon="i-lucide-person-standing"
      title="No avatar yet"
      description="Your leash shows here once VRChat tells the app which avatar you wear."
    >
      <template #actions>
        <UButton
          icon="i-lucide-book-open"
          color="neutral"
          variant="subtle"
          to="/leash/setup"
        >
          Avatar setup guide
        </UButton>
      </template>
    </EmptyState>

    <div
      v-else
      class="grid max-w-5xl grid-cols-[15rem_minmax(0,1fr)] items-start gap-3.5 max-lg:grid-cols-1"
    >
      <!-- Beside the settings, the compass stays in view while they scroll. -->
      <FormSection class="justify-items-center lg:sticky lg:top-0">
        <LeashCompass
          :forward="forward"
          :right="right"
          :stretch="state.stretch"
          :walk="avatar.walk"
          :run="avatar.run"
          :held="state.held.length > 0"
        />
        <span
          class="rounded-full px-2.5 py-1 text-[11.5px] font-semibold tracking-wider uppercase"
          :class="motion.class"
        >{{ motion.label }}</span>
        <span class="text-center font-mono text-[11.5px] text-muted tabular-nums">
          stretch {{ state.stretch.toFixed(2) }} · fwd {{ forward.toFixed(2) }} · right {{ right.toFixed(2) }}
        </span>
      </FormSection>

      <div class="grid min-w-0 gap-3.5">
        <FormSection
          title="Leashes on this avatar"
          :description="
            state.detected.leashes.length
              ? 'Only the lit ones move you. Ears, tails and other grabbable bones stay off until you pick them.'
              : 'No grabbable PhysBone with _IsGrabbed and _Stretch found.'
          "
        >
          <div
            v-if="state.detected.leashes.length"
            class="flex flex-wrap gap-1.5"
          >
            <button
              v-for="leash in state.detected.leashes"
              :key="leash"
              type="button"
              class="inline-flex items-center gap-1.5 rounded-full border px-3 py-1.5 text-[13px] transition-colors"
              :class="movesYou(avatar, leash) ? 'border-primary/50 bg-primary/14 text-highlighted' : 'border-transparent bg-(--aurora-well) text-muted hover:bg-(--aurora-glass)'"
              :aria-pressed="movesYou(avatar, leash)"
              @click="toggleLeash(leash)"
            >
              <span
                v-if="state.held.includes(leash)"
                class="size-1.75 rounded-full bg-warning"
              />
              {{ state.held.includes(leash) ? `${leash} · grabbed` : leash }}
            </button>
          </div>
        </FormSection>

        <FormSection
          v-if="state.detected.leashes.length"
          :title="`Settings for ${avatarName}`"
          description="Leash length and avatar size change what feels right, so these are kept for each avatar."
        >
          <div class="grid gap-2">
            <div class="flex items-center justify-between gap-3 text-sm">
              <span class="text-muted">Start walking at</span>
              <b class="font-semibold text-highlighted tabular-nums">{{ pct(avatar.walk) }}</b>
            </div>
            <USlider
              :model-value="Math.round(avatar.walk * 100)"
              :min="0"
              :max="95"
              :step="1"
              size="lg"
              aria-label="Start walking at"
              @update:model-value="changeAvatar((own) => (own.walk = ($event ?? 0) / 100))"
            />
          </div>
          <div class="grid gap-2">
            <div class="flex items-center justify-between gap-3 text-sm">
              <span class="text-muted">Start running at</span>
              <b class="font-semibold text-highlighted tabular-nums">{{ pct(avatar.run) }}</b>
            </div>
            <USlider
              :model-value="Math.round(avatar.run * 100)"
              :min="5"
              :max="100"
              :step="1"
              size="lg"
              aria-label="Start running at"
              @update:model-value="changeAvatar((own) => (own.run = ($event ?? 0) / 100))"
            />
          </div>
          <div class="grid gap-2">
            <div class="flex items-center justify-between gap-3 text-sm">
              <span class="text-muted">Strength</span>
              <b class="font-semibold text-highlighted tabular-nums">{{ avatar.strength.toFixed(1) }}×</b>
            </div>
            <USlider
              :model-value="Math.round(avatar.strength * 10)"
              :min="5"
              :max="30"
              :step="1"
              size="lg"
              aria-label="Strength"
              @update:model-value="changeAvatar((own) => (own.strength = ($event ?? 10) / 10))"
            />
          </div>
          <div class="grid gap-2">
            <div class="flex items-center justify-between gap-3 text-sm">
              <span class="text-muted">Stop if pulled up or down past</span>
              <b class="font-semibold text-highlighted tabular-nums">{{ pct(avatar.upDownDeadzone) }}</b>
            </div>
            <USlider
              :model-value="Math.round(avatar.upDownDeadzone * 100)"
              :min="10"
              :max="100"
              :step="1"
              size="lg"
              aria-label="Stop if pulled up or down past"
              @update:model-value="changeAvatar((own) => (own.upDownDeadzone = ($event ?? 50) / 100))"
            />
          </div>

          <div class="flex items-center gap-3 rounded-field bg-(--aurora-well) px-3 py-2.5">
            <span class="grid min-w-0 flex-1">
              <b class="text-[14.5px] font-semibold text-highlighted">Turn toward the pull</b>
              <small class="text-xs text-muted">Can cause motion sickness</small>
            </span>
            <USwitch
              :model-value="avatar.turning.enabled"
              aria-label="Turn toward the pull"
              @update:model-value="setTurning"
            />
          </div>
          <template v-if="avatar.turning.enabled">
            <div class="grid gap-2">
              <div class="flex items-center justify-between gap-3 text-sm">
                <span class="text-muted">Turning speed</span>
                <b class="font-semibold text-highlighted tabular-nums">{{ avatar.turning.multiplier.toFixed(1) }}×</b>
              </div>
              <USlider
                :model-value="Math.round(avatar.turning.multiplier * 10)"
                :min="1"
                :max="20"
                :step="1"
                size="lg"
                aria-label="Turning speed"
                @update:model-value="changeAvatar((own) => (own.turning.multiplier = ($event ?? 8) / 10))"
              />
            </div>
            <div class="grid gap-2">
              <div class="flex items-center justify-between gap-3 text-sm">
                <span class="text-muted">Start turning at</span>
                <b class="font-semibold text-highlighted tabular-nums">{{ pct(avatar.turning.deadzone) }}</b>
              </div>
              <USlider
                :model-value="Math.round(avatar.turning.deadzone * 100)"
                :min="0"
                :max="95"
                :step="1"
                size="lg"
                aria-label="Start turning at"
                @update:model-value="changeAvatar((own) => (own.turning.deadzone = ($event ?? 15) / 100))"
              />
            </div>
            <div class="grid gap-2">
              <div class="flex items-center justify-between gap-3 text-sm">
                <span class="text-muted">Stop turning within</span>
                <b class="font-semibold text-highlighted tabular-nums">{{ Math.round(avatar.turning.goalDegrees) }}°</b>
              </div>
              <USlider
                :model-value="Math.round(avatar.turning.goalDegrees)"
                :min="0"
                :max="180"
                :step="5"
                size="lg"
                aria-label="Stop turning within"
                @update:model-value="changeAvatar((own) => (own.turning.goalDegrees = $event ?? 90))"
              />
            </div>
            <p
              v-if="unnamedHang.length"
              class="text-[13px] text-muted"
            >
              {{ unnamedHang.join(', ') }} {{ unnamedHang.length === 1 ? 'turns' : 'turn' }} as if the leash is in front of you. End a leash's name with
              <code>_North</code>, <code>_South</code>, <code>_East</code> or <code>_West</code> to say which way it hangs.
            </p>
          </template>
        </FormSection>

        <FormSection
          title="Setup checks"
          description="The app only sees your avatar's parameters and their values. The last checks need someone to pull the leash, or grab it yourself."
        >
          <ul class="grid gap-2">
            <li
              v-for="row in checks"
              :key="row.key"
              class="grid grid-cols-[1.25rem_minmax(0,1fr)] gap-2 text-[13px]"
            >
              <UIcon
                :name="CHECK_ICON[row.check].name"
                class="mt-0.5 size-4"
                :class="CHECK_ICON[row.check].class"
              />
              <span class="text-muted"><b class="font-semibold text-highlighted">{{ row.label }}</b> · {{ row.detail }}</span>
            </li>
          </ul>
        </FormSection>

        <div class="flex flex-wrap gap-2">
          <UButton
            icon="i-lucide-footprints"
            :loading="testing"
            :disabled="!settings.enabled"
            @click="testPull"
          >
            Test pull
          </UButton>
          <UButton
            icon="i-lucide-book-open"
            color="neutral"
            variant="subtle"
            to="/leash/setup"
          >
            Avatar setup guide
          </UButton>
        </div>
        <p class="-mt-1.5 text-[13px] text-muted">
          <b class="font-semibold text-highlighted">Test pull</b> walks you forward for one second, so you can check the direction without a second person.
        </p>

        <FormSection
          title="Avatar parameters"
          description="Add this to your avatar to use it in VRChat."
        >
          <dl class="grid gap-2 text-sm">
            <div class="grid gap-0.5 rounded-field bg-(--aurora-well) px-3.5 py-2.5">
              <dt><code class="font-semibold text-highlighted">TKOSC/Leash</code> <span class="text-muted">· bool</span></dt>
              <dd class="text-muted">
                Turns the leash on and off from your expression menu, like <b class="text-highlighted">Leash moves me</b> here.
              </dd>
            </div>
          </dl>
        </FormSection>
      </div>
    </div>
  </PageLayout>
</template>
