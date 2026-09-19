<script setup lang="ts">
import AppHeader from '@renderer/components/AppHeader.vue'
import ControlGroup from '@renderer/components/controls/ControlGroup.vue'
import TitleBar from '@renderer/components/TitleBar.vue'
import { useAiAccess } from '@renderer/composables/useAiAccess'
import { useAiControlsModal } from '@renderer/composables/useAiControlsModal'
import { useAppUpdater } from '@renderer/composables/useAppUpdater'
import { useAvatarDetails } from '@renderer/composables/useAvatarDetails'
import { useControls } from '@renderer/composables/useControls'
import { useLogRetention } from '@renderer/composables/useLogRetention'
import { useOscConnection } from '@renderer/composables/useOscConnection'
import { useTheme } from '@renderer/composables/useTheme'
import { api } from '@renderer/lib/tauri-bridge'
import type { ControlGroup as ControlGroupType } from '@vrc-osc-toolkit/shared-ui'
import { computed, onMounted } from 'vue'
import { VueDraggable } from 'vue-draggable-plus'

const { controls, setGroups, addGroup } = useControls()
const { openModal: openAiControlsModal } = useAiControlsModal()
const { loadTheme } = useTheme()
const { connected: oscConnected } = useOscConnection()
const { avatarDetails } = useAvatarDetails()
const { access: aiAccess } = useAiAccess()

const groups = computed<ControlGroupType[]>({
  get: () => controls.value,
  set: (value) => setGroups(value)
})

// Control groups are keyed by avatar id (see useControls.ts's loadControls/saveControls), so
// there's nothing meaningful to show or add to until VRChat's OSC is both alive and has told us
// which avatar is loaded.
const ready = computed(() => oscConnected.value && !!avatarDetails.value)

onMounted(() => api.ready())
onMounted(() => loadTheme())
useLogRetention()
useAppUpdater()
</script>

<template>
  <UApp>
    <div class="flex h-dvh flex-col">
      <TitleBar />

      <div class="flex flex-1 flex-col gap-4 overflow-auto p-4">
        <AppHeader />

        <div
          v-if="ready"
          class="flex flex-col gap-4"
        >
          <!-- forceFallback: same reasoning as the per-control VueDraggable inside ControlGroup.vue
          - the native HTML5 drag API's own "not a valid drop target" cursor shows up otherwise. No
          `group` prop here (unlike that inner one): groups don't drop into one another, only
          reorder among themselves.

          CSS multi-column masonry, not flex-wrap: groups vary wildly in height (a 2-control group
          next to a 20-control one), and flex-wrap's row-based wrapping sizes every row to its
          tallest card, leaving dead space under every shorter one in that row. `columns-*` instead
          packs each card into whichever column is next in DOM order, stacking top-to-bottom per
          column - real masonry, no JS layout pass needed. Sortable's drag-and-drop doesn't care
          which CSS layout mode arranges its children, so reordering still works unchanged. -->
          <VueDraggable
            v-model.lazy="groups"
            handle=".group-handle"
            easing="cubic-bezier(0.25, 0.8, 0.25, 1)"
            :animation="200"
            :force-fallback="true"
            class="columns-1 gap-4 sm:columns-2 xl:columns-3"
          >
            <ControlGroup
              v-for="controlGroup in groups"
              :key="controlGroup.id"
              class="mx-auto mb-4 break-inside-avoid"
              :ui="{ body: 'p-4 sm:p-4 space-y-4' }"
              :control-group="controlGroup"
            />
          </VueDraggable>

          <div class="flex justify-center gap-2">
            <UButton
              icon="i-lucide-plus"
              @click="addGroup"
            >
              Add Group
            </UButton>
            <button
              v-if="aiAccess.hasAccess"
              type="button"
              class="ai-generate-btn"
              @click="openAiControlsModal"
            >
              <UIcon
                name="i-lucide-sparkles"
                class="size-4"
              />
              AI Generate
            </button>
          </div>
        </div>

        <UCard
          v-else
          class="mx-auto w-full max-w-lg"
        >
          <div class="flex flex-col items-center gap-3 py-4 text-center">
            <UIcon
              name="i-lucide-radio-tower"
              class="size-10 text-muted"
            />
            <div>
              <p class="font-medium">
                {{ oscConnected ? 'Waiting for an avatar' : 'VRChat not detected' }}
              </p>
              <p class="text-sm text-muted">
                {{
                  oscConnected
                    ? "OSC is connected, but no avatar has been detected yet. Load into a world and it'll show up here."
                    : "No OSC data has been received yet. Make sure VRChat is running and OSC is enabled."
                }}
              </p>
            </div>

            <div class="w-full space-y-1.5 rounded-md bg-elevated/50 p-3 text-left text-sm">
              <p class="font-medium">
                {{ oscConnected ? "Avatar not showing up?" : "How to enable OSC in VRChat" }}
              </p>
              <!-- VRChat only sends avatar info once, right as OSC is enabled - if that happened
              before this app was listening (or the message just got missed), nothing here will
              ever populate `avatarDetails` until OSC is toggled off and back on to resend it. -->
              <p
                v-if="oscConnected"
                class="text-muted"
              >
                Toggle OSC off, then back on, via
                <span class="text-default">Radial Menu → Options → OSC → Enabled</span>
                or <span class="text-default">Settings → Avatars → OSC</span> - this makes VRChat
                resend your avatar's info.
              </p>
              <ol
                v-else
                class="list-decimal space-y-1 pl-5 text-muted"
              >
                <li>Launch VRChat and load into any world.</li>
                <li>
                  Enable OSC, either via
                  <span class="text-default">Radial Menu → Options → OSC → Enabled</span>
                  or via
                  <span class="text-default">Settings → Avatars → OSC</span>.
                </li>
              </ol>
            </div>
          </div>
        </UCard>
      </div>
    </div>
  </UApp>
</template>

<style>
/* A themed (not hardcoded-neon) gradient so it fits whatever primary/secondary color the user has
   picked in Settings (see useTheme.ts) - the same --ui-color-* vars main.css already reads for the
   scrollbar. Sits alongside UButton rather than using its `color`/`variant` props since neither
   supports an animated multi-stop gradient background. */
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
    background-position 0.6s ease;
}

.ai-generate-btn:hover {
  background-position: 100% 0%;
  transform: translateY(-1px);
  box-shadow: 0 4px 14px color-mix(in oklab, var(--ui-color-primary-500) 45%, transparent);
}

.ai-generate-btn:active {
  transform: translateY(0);
}
</style>
