<script setup lang="ts">
import logoUrl from '@renderer/assets/logo.svg'
import ReleaseNotes from '@renderer/components/settings/ReleaseNotes.vue'
import EmptyState from '@renderer/components/ui/EmptyState.vue'
import FormSection from '@renderer/components/ui/FormSection.vue'
import { useAppUpdater } from '@renderer/composables/useAppUpdater'
import { useChangelog } from '@renderer/composables/useChangelog'
import { api } from '@renderer/lib/tauri-bridge'
import { getVersion } from '@tauri-apps/api/app'
import { computed, onMounted, ref } from 'vue'

// Update results (up to date, update available with an Update & Restart action, failures) are
// shown as toasts by useAppUpdater itself.
const { checking, installing, pendingUpdate, installPendingUpdate, checkForUpdates } = useAppUpdater()
const { releases, currentVersion, loading, error, load } = useChangelog()
const version = ref<string>()

// A dev build never installs an update (see useAppUpdater.ts), so it doesn't offer the button.
const DEV_BUILD = import.meta.env.DEV

onMounted(() => void getVersion().then((value) => (version.value = value)))
// The notes are only fetched once you open this section, not at launch.
onMounted(() => void load())

// Everything you'd get by updating now - you may be several versions behind.
const newerReleases = computed(() => releases.value.filter((r) => r.isNewer))

// Your version first and open, older ones collapsed below it.
const pastReleases = computed(() =>
  releases.value
    .filter((r) => !r.isNewer)
    .map((r) => ({
      label: `v${r.version}${r.version === currentVersion.value ? ' (installed)' : ''}`,
      value: r.version,
      release: r
    }))
)
const openReleases = ref<string[]>([])
const accordionValue = computed({
  get: () => (openReleases.value.length || !pastReleases.value[0] ? openReleases.value : [pastReleases.value[0].value]),
  set: (value: string[]) => (openReleases.value = value)
})

const formatDate = (date: string): string =>
  date ? new Date(date).toLocaleDateString(undefined, { year: 'numeric', month: 'long', day: 'numeric' }) : ''
</script>

<template>
  <FormSection>
    <div class="flex flex-wrap items-center gap-3.5">
      <img
        :src="logoUrl"
        alt=""
        class="size-12 rounded-xl"
      >
      <div class="grid min-w-0 flex-1">
        <b class="text-[15px] font-semibold text-highlighted">ToolKitOSC</b>
        <small class="text-[13px] text-muted">{{ version ? `Version ${version}` : 'Checking version…' }}</small>
      </div>
      <UButton
        icon="i-lucide-refresh-cw"
        color="neutral"
        variant="subtle"
        :loading="checking"
        @click="checkForUpdates()"
      >
        Check for updates
      </UButton>
    </div>
  </FormSection>

  <FormSection
    title="Credits"
    description="ToolKitOSC builds on other people's work."
  >
    <RouterLink
      to="/settings/about/credits"
      class="flex min-h-14 items-center gap-3 rounded-field bg-(--aurora-well) px-3.5 py-2.5 transition-colors hover:bg-(--aurora-glass)"
    >
      <UIcon
        name="i-lucide-heart-handshake"
        class="size-6 shrink-0 text-primary"
      />
      <span class="grid min-w-0 flex-1 gap-0.5 text-sm">
        <b class="font-semibold text-highlighted">Credits & licenses</b>
        <small class="text-[13px] text-muted">OpenShock, Intiface, OSCLeash, OscGoesBrrr and every open-source library inside the app.</small>
      </span>
      <UIcon
        name="i-lucide-chevron-right"
        class="size-4 shrink-0 text-dimmed"
      />
    </RouterLink>
  </FormSection>

  <FormSection
    v-if="newerReleases.length"
    id="update-notes"
    class="scroll-mt-3.5"
    :title="`Update to v${newerReleases[0]!.version}`"
    :description="
      DEV_BUILD
        ? 'Development builds don\'t install updates. Install it from a release build.'
        : newerReleases.length > 1
          ? `What changes since v${currentVersion}, across ${newerReleases.length} versions.`
          : `What changes since v${currentVersion}.`
    "
  >
    <template
      v-if="pendingUpdate && !DEV_BUILD"
      #actions
    >
      <UButton
        icon="i-lucide-download"
        color="primary"
        :loading="installing"
        @click="installPendingUpdate()"
      >
        Update & Restart
      </UButton>
    </template>

    <article
      v-for="release in newerReleases"
      :key="release.version"
      class="grid gap-2 rounded-field bg-(--aurora-well) p-4"
    >
      <header class="flex flex-wrap items-baseline gap-x-2.5">
        <b class="text-[15px] font-semibold text-highlighted">v{{ release.version }}</b>
        <small class="text-xs text-muted">{{ formatDate(release.publishedAt) }}</small>
      </header>
      <ReleaseNotes :html="release.html" />
    </article>
  </FormSection>

  <FormSection
    id="release-notes"
    class="scroll-mt-3.5"
    title="Release notes"
    description="What changed in each version of ToolKitOSC."
  >
    <template #actions>
      <UButton
        icon="i-lucide-external-link"
        color="neutral"
        variant="ghost"
        @click="api.openUrl('https://github.com/NekoTiki/ToolKitOSC/releases')"
      >
        All releases
      </UButton>
    </template>

    <div
      v-if="loading && !releases.length"
      class="grid gap-2.5"
    >
      <USkeleton
        v-for="n in 3"
        :key="n"
        class="h-12 rounded-field"
      />
    </div>

    <EmptyState
      v-else-if="error"
      icon="i-lucide-cloud-off"
      title="Couldn't load the release notes"
      description="GitHub couldn't be reached. Check your connection and try again."
    >
      <template #actions>
        <UButton
          icon="i-lucide-refresh-cw"
          color="neutral"
          variant="subtle"
          :loading="loading"
          @click="load({ force: true })"
        >
          Retry
        </UButton>
      </template>
    </EmptyState>

    <EmptyState
      v-else-if="!pastReleases.length"
      icon="i-lucide-scroll-text"
      title="No release notes yet"
    />

    <UAccordion
      v-else
      v-model="accordionValue"
      type="multiple"
      :items="pastReleases"
      :ui="{ trigger: 'min-h-12 text-[15px] font-semibold', body: 'pb-4' }"
    >
      <template #trailing="{ item }">
        <small class="ms-auto text-xs font-normal text-muted">{{ formatDate(item.release.publishedAt) }}</small>
      </template>
      <template #body="{ item }">
        <ReleaseNotes :html="item.release.html" />
      </template>
    </UAccordion>
  </FormSection>
</template>
