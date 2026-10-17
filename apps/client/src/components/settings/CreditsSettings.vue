<script setup lang="ts">
import EmptyState from '@renderer/components/ui/EmptyState.vue'
import FormSection from '@renderer/components/ui/FormSection.vue'
import SegmentedControl from '@renderer/components/ui/SegmentedControl.vue'
import { useCredits } from '@renderer/composables/useCredits'
import type { CreditPackage } from '@renderer/lib/credits'
import { api } from '@renderer/lib/tauri-bridge'
import { computed, onMounted, ref } from 'vue'

// Settings › About › Credits, a sub-page of About: the projects ToolKitOSC owes the most to, then every library the
// app ships with its license, from the list written at build time (see vite-plugin-credits.ts).
const { credits, loading, error, load } = useCredits()
onMounted(() => void load())

const THANKS = [
  {
    name: 'OpenShock',
    by: 'the OpenShock team',
    text: 'Your shockers are controlled through its API.',
    links: [
      { label: 'Website', url: 'https://openshock.org' },
      { label: 'GitHub', url: 'https://github.com/OpenShock' }
    ]
  },
  {
    name: 'Intiface',
    by: 'Nonpolynomial',
    text: 'Intiface Central and its Buttplug protocol connect your toys.',
    links: [
      { label: 'Website', url: 'https://intiface.com' },
      { label: 'Buttplug', url: 'https://buttplug.io' }
    ]
  },
  {
    name: 'OSCLeash',
    by: 'ZenithVal · MIT',
    text: 'The avatar leash uses its compass prefab and its maths.',
    links: [{ label: 'GitHub', url: 'https://github.com/ZenithVal/OSCLeash' }]
  },
  {
    name: 'OscGoesBrrr',
    by: 'SenkyDragon',
    text: 'Avatar haptics read the same OGB contacts, so avatars set up for it work as they are.',
    links: [
      { label: 'Website', url: 'https://osc.toys' },
      { label: 'GitHub', url: 'https://github.com/OscToys/OscGoesBrrr' }
    ]
  }
]

type Kind = 'all' | 'rust' | 'js'
interface Entry extends CreditPackage {
  key: string
  kind: 'Rust' | 'JavaScript'
}

const entries = computed<Entry[]>(() =>
  credits.value
    ? [
        ...credits.value.rust.map((p) => ({ ...p, key: `rust:${p.name}@${p.version}`, kind: 'Rust' as const })),
        ...credits.value.js.map((p) => ({ ...p, key: `js:${p.name}@${p.version}`, kind: 'JavaScript' as const }))
      ].sort((a, b) => a.name.localeCompare(b.name))
    : []
)

const kind = ref<Kind>('all')
const search = ref('')
const kindItems = computed(() => [
  { value: 'all' as const, label: `All ${entries.value.length}` },
  { value: 'rust' as const, label: `Rust ${credits.value?.rust.length ?? 0}` },
  { value: 'js' as const, label: `JavaScript ${credits.value?.js.length ?? 0}` }
])

// Name, license or author.
const shown = computed(() => {
  const query = search.value.trim().toLowerCase()
  return entries.value.filter(
    (e) =>
      (kind.value === 'all' || (kind.value === 'rust') === (e.kind === 'Rust')) &&
      (!query || [e.name, e.license ?? '', ...e.authors].some((s) => s.toLowerCase().includes(query)))
  )
})

const open = ref(new Set<string>())
const toggle = (key: string): void => {
  if (open.value.has(key)) open.value.delete(key)
  else open.value.add(key)
}

const DEV_BUILD = import.meta.env.DEV
</script>

<template>
  <FormSection
    title="Special thanks"
    description="ToolKitOSC builds on these projects."
  >
    <div class="grid gap-2.5 sm:grid-cols-2 xl:grid-cols-4">
      <article
        v-for="project in THANKS"
        :key="project.name"
        class="flex flex-col gap-2 rounded-field bg-(--aurora-well) px-3.5 py-3"
      >
        <span class="grid gap-0.5 text-sm">
          <b class="font-semibold text-highlighted">{{ project.name }} <span class="font-normal text-muted">by {{ project.by }}</span></b>
          <small class="text-[13px] text-muted">{{ project.text }}</small>
        </span>
        <div class="mt-auto flex flex-wrap gap-1.5">
          <UButton
            v-for="link in project.links"
            :key="link.url"
            icon="i-lucide-external-link"
            color="neutral"
            variant="subtle"
            size="md"
            @click="api.openUrl(link.url)"
          >
            {{ link.label }}
          </UButton>
        </div>
      </article>
    </div>
  </FormSection>

  <FormSection
    title="Open-source libraries"
    :description="
      credits
        ? `${credits.rust.length} Rust crates and ${credits.js.length} JavaScript packages ship inside ToolKitOSC. Open one to read its license.`
        : 'Every library that ships inside ToolKitOSC, with its license.'
    "
  >
    <div
      v-if="loading && !credits"
      class="grid gap-2.5"
    >
      <USkeleton
        v-for="n in 4"
        :key="n"
        class="h-14 rounded-field"
      />
    </div>

    <EmptyState
      v-else-if="error"
      icon="i-lucide-file-x"
      title="Couldn't load the list"
      description="The credits file is missing from this build."
    >
      <template #actions>
        <UButton
          icon="i-lucide-refresh-cw"
          color="neutral"
          variant="subtle"
          :loading="loading"
          @click="load()"
        >
          Retry
        </UButton>
      </template>
    </EmptyState>

    <template v-else-if="credits">
      <p
        v-if="DEV_BUILD && credits.partialJs"
        class="text-[13px] text-warning"
      >
        Development build: only the client's direct JavaScript dependencies are listed. A release build lists everything in its bundle.
      </p>

      <div class="flex flex-wrap items-center gap-2.5">
        <UInput
          v-model="search"
          icon="i-lucide-search"
          placeholder="Search by name, license or author"
          class="min-w-48 flex-1"
        />
        <SegmentedControl
          v-model="kind"
          :items="kindItems"
          aria-label="Language"
        />
      </div>

      <EmptyState
        v-if="!shown.length"
        icon="i-lucide-search-x"
        title="No library matches"
      />

      <ul
        v-else
        class="grid gap-1.5"
      >
        <li
          v-for="entry in shown"
          :key="entry.key"
          class="rounded-field bg-(--aurora-well)"
        >
          <button
            type="button"
            class="flex min-h-14 w-full cursor-pointer items-center gap-3 px-3.5 py-2 text-left"
            :aria-expanded="open.has(entry.key)"
            @click="toggle(entry.key)"
          >
            <span class="grid min-w-0 flex-1 gap-0.5">
              <b class="truncate text-sm font-semibold text-highlighted">
                {{ entry.name }} <span class="font-normal text-muted">{{ entry.version }}</span>
              </b>
              <small
                v-if="entry.description"
                class="truncate text-[13px] text-muted"
              >{{ entry.description }}</small>
            </span>
            <UBadge
              v-if="entry.license"
              color="neutral"
              variant="subtle"
              class="max-w-40 shrink-0 truncate"
            >
              {{ entry.license }}
            </UBadge>
            <UIcon
              name="i-lucide-chevron-down"
              class="size-4 shrink-0 text-dimmed transition-transform"
              :class="open.has(entry.key) ? 'rotate-180' : undefined"
            />
          </button>

          <div
            v-if="open.has(entry.key)"
            class="grid gap-2.5 border-t border-default px-3.5 py-3"
          >
            <p class="text-[13px] text-muted">
              {{ entry.kind }}<template v-if="entry.authors.length">
                · by {{ entry.authors.join(', ') }}
              </template>
            </p>
            <div
              v-if="entry.repository || entry.homepage"
              class="flex flex-wrap gap-1.5"
            >
              <UButton
                v-if="entry.repository"
                icon="i-lucide-external-link"
                color="neutral"
                variant="subtle"
                size="md"
                @click="api.openUrl(entry.repository)"
              >
                Repository
              </UButton>
              <UButton
                v-if="entry.homepage"
                icon="i-lucide-external-link"
                color="neutral"
                variant="subtle"
                size="md"
                @click="api.openUrl(entry.homepage)"
              >
                Homepage
              </UButton>
            </div>
            <pre
              v-for="index in entry.licenseTexts"
              :key="index"
              class="max-h-72 overflow-auto rounded-xl bg-(--aurora-glass) p-3 font-mono text-xs whitespace-pre-wrap text-muted"
            >{{ credits.licenseTexts[index] }}</pre>
            <p
              v-if="!entry.licenseTexts.length"
              class="text-[13px] text-muted"
            >
              This package ships no license file. {{ entry.license ? `It's published under ${entry.license}.` : '' }}
            </p>
          </div>
        </li>
      </ul>
    </template>
  </FormSection>

  <p class="px-1 text-[13px] text-muted">
    ToolKitOSC itself is free software under the GPL-3.0-or-later license.
    <button
      type="button"
      class="cursor-pointer text-primary underline underline-offset-3"
      @click="api.openUrl('https://github.com/NekoTiki/ToolKitOSC/blob/master/LICENSE')"
    >
      Read the license
    </button>
  </p>
</template>
