<script setup lang="ts">
import FormSection from '@renderer/components/ui/FormSection.vue'
import { api } from '@renderer/lib/tauri-bridge'

// Leash > Avatar setup guide: what an avatar needs for the leash to move you. The avatar side is
// OSCLeash's prefab, unchanged, so avatars already set up for OSCLeash work as they are. The prefab
// brings everything (the compass and its own leash PhysBone), so this only says what to do with it
// and points to a video that shows it.
const OSCLEASH_URL = 'https://github.com/ZenithVal/OSCLeash'
const OSCLEASH_RELEASES_URL = 'https://github.com/ZenithVal/OSCLeash/releases'
const TUTORIAL_URL = 'https://www.youtube.com/watch?v=t7OBOoDCRQ4'

const PARAMETERS = [
  { name: 'Leash_IsGrabbed', type: 'Bool', meaning: 'Someone holds the leash' },
  { name: 'Leash_Stretch', type: 'Float', meaning: "How far it's pulled, 0 to 1" },
  { name: 'Leash_Z+ / Leash_Z-', type: 'Float', meaning: 'Forward / back share of the pull' },
  { name: 'Leash_X+ / Leash_X-', type: 'Float', meaning: 'Right / left' },
  { name: 'Leash_Y+ / Leash_Y-', type: 'Float', meaning: 'Up / down' },
  { name: 'TKOSC/Leash', type: 'Bool', meaning: 'Optional: turns the leash on and off from your menu' }
]
</script>

<template>
  <PageLayout>
    <template #header>
      <PageHeader
        title="Avatar setup guide"
        :crumbs="[{ label: 'Leash', to: '/leash' }, { label: 'Avatar setup guide' }]"
      />
    </template>

    <div class="grid max-w-3xl gap-3.5">
      <UAlert
        color="warning"
        variant="subtle"
        icon="i-lucide-triangle-alert"
        title="Close OSCLeash"
        description="If the OSCLeash app still runs, both apps move you. ToolKitOSC does its job now, with the same avatar setup."
      />

      <FormSection
        title="What your avatar needs"
        description="Everything comes with OSCLeash's prefab: the compass that tells the app which way you're pulled, and a leash PhysBone already set up for it. That PhysBone replaces the one on your own leash."
      >
        <template #actions>
          <UButton
            icon="i-lucide-external-link"
            color="neutral"
            variant="subtle"
            @click="api.openUrl(OSCLEASH_RELEASES_URL)"
          >
            Get the prefab
          </UButton>
        </template>
        <ol class="grid list-decimal gap-2 pl-5 text-sm text-muted marker:text-dimmed">
          <li>
            Download the prefab from <b class="text-highlighted">OSCLeash</b>'s releases and add it to your avatar.
          </li>
          <li>
            Turn off or remove the PhysBone on your own leash. The prefab's PhysBone takes its place, and two of them on the same bones fight each other.
          </li>
          <li>
            Fit the prefab to your leash and avatar. The video below shows each step.
          </li>
          <li>
            Upload, wear the avatar and open the <b class="text-highlighted">Leash</b> page: its setup checks say what's still missing.
          </li>
        </ol>
        <button
          type="button"
          class="flex items-center gap-3 rounded-field bg-(--aurora-well) px-3.5 py-2.5 text-left transition-colors hover:bg-(--aurora-glass)"
          @click="api.openUrl(TUTORIAL_URL)"
        >
          <UIcon
            name="i-lucide-circle-play"
            class="size-6 shrink-0 text-primary"
          />
          <span class="grid min-w-0 flex-1 gap-0.5">
            <b class="text-sm font-semibold text-highlighted">Thorough OSCLeash tutorial for VRChat Avatars</b>
            <small class="text-[13px] text-muted">Video by Meeman 7 on YouTube</small>
          </span>
          <UIcon
            name="i-lucide-external-link"
            class="size-4 shrink-0 text-dimmed"
          />
        </button>
      </FormSection>

      <FormSection
        title="Parameters"
        description="What the app reads from your avatar. They're unsynced, so they cost no bits. A leash with another name uses that name for _IsGrabbed and _Stretch; the six compass ones stay Leash_…"
      >
        <div class="overflow-x-auto rounded-field bg-(--aurora-well)">
          <table class="w-full text-left text-sm">
            <thead class="text-xs text-muted">
              <tr>
                <th class="px-3.5 py-2 font-medium">
                  Parameter
                </th>
                <th class="px-3.5 py-2 font-medium">
                  Type
                </th>
                <th class="px-3.5 py-2 font-medium">
                  Meaning
                </th>
              </tr>
            </thead>
            <tbody>
              <tr
                v-for="parameter in PARAMETERS"
                :key="parameter.name"
                class="border-t border-default"
              >
                <td class="px-3.5 py-2 whitespace-nowrap">
                  <code class="font-semibold text-highlighted">{{ parameter.name }}</code>
                </td>
                <td class="px-3.5 py-2 text-muted">
                  {{ parameter.type }}
                </td>
                <td class="px-3.5 py-2 text-muted">
                  {{ parameter.meaning }}
                </td>
              </tr>
            </tbody>
          </table>
        </div>
        <p class="text-[13px] text-muted">
          Several leashes work too, e.g. <code>Leash_North</code> and <code>Leash_South</code>. The ending says which way the leash hangs, for turning.
        </p>
      </FormSection>

      <FormSection
        title="Thanks to OSCLeash"
        description="The compass prefab, and the maths that turns a pull into walking, come from OSCLeash by ZenithVal (MIT licence)."
      >
        <template #actions>
          <UButton
            icon="i-lucide-external-link"
            color="neutral"
            variant="subtle"
            @click="api.openUrl(OSCLEASH_URL)"
          >
            OSCLeash on GitHub
          </UButton>
        </template>
      </FormSection>
    </div>
  </PageLayout>
</template>
