<script setup lang="ts">
import { Control, useMasonry } from '@vrc-osc-toolkit/shared-ui'

import { useClientTheme } from '~/composables/useClientTheme'
import type { ShareInfo } from '~~/server/api/share/[shareId].get'

const shareId = useRoute().params.shareId as string

// Awaited (not fire-and-forget) so `shareInfo` is already populated when the SEO meta below reads
// it - this is what Discord's link-preview crawler (which never runs JS) actually sees, not the
// generic fallback text a client-side-only fetch would leave in the initial HTML.
const { data: shareInfo } = await useFetch<ShareInfo>(`/api/share/${shareId}`)

const requestUrl = useRequestURL()
const ogImageUrl = `${requestUrl.origin}/og-image.png`
const pageUrl = `${requestUrl.origin}/share/${shareId}`

const title = computed(() =>
  shareInfo.value?.hostName ? `${shareInfo.value.hostName}'s Controls` : 'Shared Controls'
)

const description = computed(() => {
  const info = shareInfo.value
  const who = info?.hostName ? `${info.hostName} is` : 'This host is'

  if (!info?.online) return `${who} currently offline.`
  if (info.controlCount === 0) return `${who} online, but hasn't shared any controls yet.`

  return `${who} sharing ${info.controlCount} live ${info.controlCount === 1 ? 'control' : 'controls'} - open the link to view and use them.`
})

useSeoMeta({
  title,
  description,
  ogTitle: title,
  ogDescription: description,
  ogImage: ogImageUrl,
  ogUrl: pageUrl,
  ogType: 'website',
  twitterCard: 'summary_large_image',
  twitterTitle: title,
  twitterDescription: description,
  twitterImage: ogImageUrl
})

const { controlGroups, authRequired, banned, status, hostStatus, open, close, sendMessage } =
  useWebsocketClient(shareId)
const { removeTheme } = useClientTheme()

const connectedOnce = ref(false)

const authRequiredOpen = computed({
  get: () => authRequired.value !== null,
  set: (value) => {
    if (!value) authRequired.value = null
  }
})

const bannedOpen = computed({
  get: () => banned.value !== null,
  set: (value) => {
    if (!value) banned.value = null
  }
})

const wsOffline = computed(() => status.value === 'CLOSED')

useMasonry(useTemplateRef('groupsGrid'))

const handleChangeUsername = (username: string) => {
  sendMessage('update-username', { displayName: username })
  $fetch('/auth/set-username', {
    method: 'POST',
    body: { username }
  })

  authRequired.value = null
}

watch(status, (newStatus) => {
  if (newStatus === 'OPEN') connectedOnce.value = true
})

onMounted(open)
onBeforeUnmount(close)
onBeforeUnmount(removeTheme)
</script>

<template>
  <UMain class="gap-4 flex flex-col py-4">
    <div
      class="flex flex-wrap justify-center items-start gap-4"
      :class="{ grow: !controlGroups.length }"
    >
      <UEmpty
        v-if="status === 'OPEN' && hostStatus === 'online' && controlGroups.length === 0"
        icon="lucide:circle-slash-2"
        title="No Controls"
        description="There are no controls available to display."
        class="h-min self-center min-w-92"
      />
      <UEmpty
        v-else-if="status === 'OPEN' && hostStatus !== 'online' && controlGroups.length === 0"
        icon="lucide:wifi-off"
        title="Host Offline"
        description="The host is currently offline. Please try again later."
        class="h-min self-center min-w-92"
        :ui="{ avatar: 'animate-pulse' }"
      />
      <UEmpty
        v-else-if="wsOffline && connectedOnce"
        icon="lucide:cloud-off"
        title="Disconnected"
        class="h-min self-center min-w-92"
      >
        <template #description>
          You are currently disconnected.<br />Please check your connection.
        </template>
        <template #actions>
          <UButton
            color="primary"
            icon="lucide:refresh-cw"
            label="Reconnect"
            variant="subtle"
            @click="open"
          />
        </template>
      </UEmpty>
      <UEmpty
        v-else-if="!connectedOnce"
        icon="lucide:loader-2"
        title="Connecting..."
        class="h-min self-center min-w-92"
        :ui="{ avatar: 'animate-spin' }"
      />
      <!-- Same masonry layout as the desktop client's own control panel (App.vue, via shared-ui's
      useMasonry) - each card keeps its definite, breakpoint-driven width (see the UCollapsible's
      w-108/2xl:w-164 below) and only gets packed into whichever column is shortest. Its own
      container (not the outer flex one above) so the empty/offline states above stay laid out as
      before; the flex-wrap classes are just the pre-mount fallback. -->
      <div
        v-else
        ref="groupsGrid"
        class="flex w-full flex-wrap items-start justify-center gap-4"
      >
        <UCard
          v-for="controlGroup in controlGroups"
          :key="controlGroup.id"
          class="w-fit max-w-full"
          :ui="{ root: 'h-min', body: 'p-4 sm:p-4 space-y-4' }"
        >
          <!-- A definite width (w-108/2xl:w-164), not max-w-full/w-full - snug-fits exactly 2
          controls below 2xl (2*w-52 + 1*gap-4 = 27rem = w-108) and exactly 3 at 2xl and up
          (3*w-52 + 2*gap-4 = 41rem = w-164), matching ControlGroup.vue's own reasoning in the
          desktop client - including why it's 2xl and not the narrower lg first tried (the jump
          to 3 columns lands right where the outer row also drops from 2 group cards side by side
          to 1, at lg but not at 2xl, so lg compounded two reflows into one jarring resize range)
          and why it has to be a real width, not a max-width, for the card to stay the same size
          once #content is collapsed. -->
          <UCollapsible
            v-model:open="useControlGroupOpen(controlGroup.id).value"
            class="flex w-108 2xl:w-164 flex-col gap-2"
            :ui="{ content: 'overflow-visible' }"
          >
            <template #default="{ open: cOpen }">
              <UButton
                :label="controlGroup.name"
                color="neutral"
                variant="subtle"
                trailing-icon="i-lucide-chevron-down"
                block
                :ui="{
                  trailingIcon: ['transition-transform', cOpen ? 'duration-200 rotate-180' : '']
                }"
              />
            </template>

            <!-- flex-wrap + a fixed width per control, not CSS grid's auto-fit/auto-fill - see
            ControlGroup.vue's matching comment in the desktop client: the UCollapsible above does
            have a definite width (w-108/2xl:w-164), but flex-wrap already packs fixed-width
            controls left-to-right and wraps once that width doesn't fit another one, with no
            per-breakpoint column-count math to keep in sync by hand. -->
            <template #content>
              <div class="flex flex-wrap items-center justify-start gap-4">
                <Control
                  v-for="control in controlGroup.controls"
                  :key="control.id"
                  class="w-52 shrink-0"
                  :control="control"
                  :locked="control.locked"
                  :unavailable="control.unavailable"
                  :offline="hostStatus !== 'online'"
                  :disconnected="wsOffline"
                  @command="
                    sendMessage('command', {
                      groupId: controlGroup.id,
                      controlId: control.id,
                      controlName: control.name,
                      ...$event
                    })
                  "
                />
              </div>
            </template>
          </UCollapsible>
        </UCard>
      </div>
    </div>
    <AuthModal
      v-model:open="authRequiredOpen"
      :method="authRequired"
      @update:username="handleChangeUsername"
    />
    <BannedModal
      v-model:open="bannedOpen"
      :scope="banned?.scope ?? null"
      :reason="banned?.reason"
    />
  </UMain>
</template>

<style>
html,
body {
  height: 100vh;
}
</style>
