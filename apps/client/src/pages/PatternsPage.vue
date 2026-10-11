<script setup lang="ts">
import IconSelectMenu from '@renderer/components/IconSelectMenu.vue'
import PatternRecorder from '@renderer/components/patterns/PatternRecorder.vue'
import PatternSparkline from '@renderer/components/patterns/PatternSparkline.vue'
import EmptyState from '@renderer/components/ui/EmptyState.vue'
import { useAreYouSureModal } from '@renderer/composables/useAreYouSureModal'
import { BUILTIN_PATTERNS, toDefinition } from '@renderer/composables/useIntifacePatterns'
import { PATTERN_SAMPLE_MS, useSavedPatterns } from '@renderer/composables/useSavedPatterns'
import { timeAgo } from '@renderer/utils/time'
import { computed, ref, watch } from 'vue'
import { onBeforeRouteLeave, useRoute, useRouter } from 'vue-router'

// Patterns: toy patterns you record by moving a slider (see PatternRecorder.vue). Yours are listed
// first in the sidebar, then the built-in ones, which can be played but not changed. Every Toy
// Pattern control can offer any of them (see IntifacePatternFields.vue).
const route = useRoute()
const router = useRouter()
const toast = useToast()
const { savedPatterns, getSavedPattern, addPattern, updatePattern, deletePattern } = useSavedPatterns()
const { openModal: confirm } = useAreYouSureModal()

const isNew = route.name === 'pattern-new'
const patternId = computed(() => (typeof route.params.patternId === 'string' ? route.params.patternId : null))
const pattern = computed(() => (patternId.value ? getSavedPattern(patternId.value) : undefined))
const builtin = computed(() => BUILTIN_PATTERNS.find((item) => item.id === patternId.value))

const savedDefinitions = computed(() => savedPatterns.value.map((item) => ({ saved: item, definition: toDefinition(item) })))

// /patterns alone (or a pattern that's gone) opens your first pattern, or else the first built-in.
watch(
  [savedPatterns, patternId],
  () => {
    if (route.name !== 'patterns' && (route.name !== 'pattern' || pattern.value || builtin.value)) return

    void router.replace(`/patterns/${savedPatterns.value[0]?.id ?? BUILTIN_PATTERNS[0]!.id}`)
  },
  { immediate: true }
)

interface Draft {
  name: string
  icon?: string
  samples: number[]
}

const initial = (): Draft | null => {
  if (isNew) return { name: '', icon: '', samples: [] }
  if (!pattern.value) return null

  return { name: pattern.value.name, icon: pattern.value.icon, samples: [...pattern.value.samples] }
}

const draft = ref<Draft | null>(initial())
let snapshot = JSON.stringify(draft.value)

const dirty = computed(() => !!draft.value && JSON.stringify(draft.value) !== snapshot)
const nameError = ref<string>()
const saved = ref(false)

const fmt = (samples: number): string => `${((samples * PATTERN_SAMPLE_MS) / 1000).toFixed(1)} s`

const save = (): void => {
  if (!draft.value) return

  const name = draft.value.name.trim()

  if (!name) {
    nameError.value = 'Give the pattern a name.'

    return
  }

  if (!draft.value.samples.length) return

  saved.value = true

  if (isNew) {
    const created = addPattern({ name, icon: draft.value.icon, samples: draft.value.samples })

    toast.add({ title: `Saved "${name}"`, description: 'Toy Pattern controls can offer it now.', icon: 'i-lucide-check', color: 'success' })
    void router.push(`/patterns/${created.id}`)

    return
  }

  updatePattern(patternId.value!, { name, icon: draft.value.icon, samples: draft.value.samples })
  snapshot = JSON.stringify(draft.value)
  saved.value = false
  toast.add({ title: `Saved "${name}"`, icon: 'i-lucide-check', color: 'success' })
}

const remove = async (): Promise<void> => {
  if (!pattern.value) return

  const ok = await confirm({
    title: 'Delete pattern?',
    message: `Delete **${pattern.value.name}**? Toy Pattern controls stop offering it.`,
    confirmText: 'Delete'
  })

  if (!ok) return

  deletePattern(pattern.value.id)
  saved.value = true
  void router.push('/patterns')
}

onBeforeRouteLeave(async () => {
  if (saved.value || !dirty.value) return true

  return confirm({ title: 'Discard changes?', message: 'Your changes to this pattern will be lost.', confirmText: 'Discard' })
})
</script>

<template>
  <PageLayout>
    <template #sidebar>
      <div class="flex min-h-9 shrink-0 items-center gap-2">
        <b class="flex-1 text-xl font-semibold text-highlighted">Patterns</b>
        <UButton
          icon="i-lucide-plus"
          to="/patterns/new"
        >
          New
        </UButton>
      </div>

      <nav
        aria-label="Patterns"
        class="-mr-1.5 flex min-h-0 flex-1 flex-col gap-1.5 overflow-y-auto pr-1.5"
      >
        <small class="px-1.5 pt-1 text-xs font-medium text-muted">Mine</small>
        <RouterLink
          v-for="{ saved: item, definition } in savedDefinitions"
          :key="item.id"
          :to="`/patterns/${item.id}`"
          class="flex min-h-14.5 items-center gap-2.5 rounded-2xl border px-2.5 py-2 transition-colors"
          :class="item.id === patternId ? 'border-primary/50 bg-primary/14' : 'border-transparent hover:bg-(--aurora-glass)'"
        >
          <span class="grid h-8.5 w-18 shrink-0 place-items-center rounded-xl bg-(--aurora-well)">
            <PatternSparkline
              :pattern="definition"
              mine
            />
          </span>
          <span class="grid min-w-0">
            <b class="truncate text-[14.5px] font-semibold text-highlighted">{{ item.name }}</b>
            <small class="truncate text-xs text-muted">Recorded · {{ fmt(item.samples.length) }}</small>
          </span>
        </RouterLink>
        <p
          v-if="!savedPatterns.length"
          class="px-1.5 text-sm text-muted"
        >
          None yet. Press New and record one with the slider.
        </p>

        <small class="px-1.5 pt-3 text-xs font-medium text-muted">Built-in</small>
        <RouterLink
          v-for="item in BUILTIN_PATTERNS"
          :key="item.id"
          :to="`/patterns/${item.id}`"
          class="flex min-h-12 items-center gap-2.5 rounded-2xl border px-2.5 py-1.5 transition-colors"
          :class="item.id === patternId ? 'border-primary/50 bg-primary/14' : 'border-transparent hover:bg-(--aurora-glass)'"
        >
          <span class="grid h-8.5 w-18 shrink-0 place-items-center rounded-xl bg-(--aurora-well)">
            <PatternSparkline :pattern="item" />
          </span>
          <span class="flex-1 truncate text-[14.5px] text-default">{{ item.name }}</span>
          <UIcon
            name="i-lucide-lock"
            class="size-3.5 text-dimmed"
          />
        </RouterLink>
      </nav>
    </template>

    <template
      v-if="draft || builtin"
      #header
    >
      <PageHeader
        :title="isNew ? 'New pattern' : (pattern?.name ?? builtin?.name ?? 'Pattern')"
        :meta="builtin ? 'Built-in' : pattern ? `Updated ${timeAgo(pattern.updatedAt)}` : undefined"
        :crumbs="[{ label: 'Patterns', to: '/patterns' }, { label: isNew ? 'New' : (pattern?.name ?? builtin?.name ?? '') }]"
      >
        <template
          v-if="pattern"
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

    <div
      v-if="draft"
      class="flex min-h-full flex-col gap-3.5"
    >
      <div class="grid gap-3 sm:grid-cols-[minmax(0,1fr)_16rem]">
        <UFormField
          label="Name"
          required
          :error="nameError"
        >
          <UInput
            v-model="draft.name"
            placeholder="Heartbeat, Slow build…"
            class="w-full"
            :autofocus="isNew"
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

      <PatternRecorder v-model:samples="draft.samples" />
    </div>

    <div
      v-else-if="builtin"
      class="flex min-h-full flex-col"
    >
      <PatternRecorder :builtin="builtin" />
    </div>

    <EmptyState
      v-else
      icon="i-lucide-audio-waveform"
      title="Pattern not found"
      description="It may have been deleted."
    />

    <template
      v-if="draft"
      #footer
    >
      <UButton
        color="neutral"
        variant="subtle"
        :disabled="!isNew && !dirty"
        @click="isNew ? router.push('/patterns') : (draft = initial())"
      >
        {{ isNew ? 'Cancel' : 'Revert' }}
      </UButton>
      <UButton
        icon="i-lucide-check"
        :disabled="!draft.samples.length || (!isNew && !dirty)"
        :title="!draft.samples.length ? 'Record a pattern first' : undefined"
        @click="save"
      >
        {{ isNew ? 'Save pattern' : 'Save changes' }}
      </UButton>
    </template>
  </PageLayout>
</template>
