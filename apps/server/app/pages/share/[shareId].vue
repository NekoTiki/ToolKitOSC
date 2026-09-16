<script setup lang="ts">
import { Control } from '@vrc-osc-toolkit/shared-ui'

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
      <UCard
        v-for="controlGroup in controlGroups"
        v-else
        :key="controlGroup.id"
        :ui="{ root: 'h-min', body: 'p-4 sm:p-4 space-y-4' }"
      >
        <UCollapsible
          v-model:open="useControlGroupOpen(controlGroup.id).value"
          class="flex flex-col gap-2 min-w-[90vw] sm:min-w-[456px] md:min-w-[692px]"
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

          <template #content>
            <div
              class="grid grid-cols-[repeat(2,minmax(0,180px))] sm:grid-cols-[repeat(2,minmax(0,220px))] md:grid-cols-[repeat(3,minmax(0,220px))] items-center justify-center-safe gap-4"
            >
              <Control
                v-for="control in controlGroup.controls"
                :key="control.id"
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
