<script setup lang="ts">
import { Control } from '@vrc-osc-toolkit/shared-ui'

import { useClientTheme } from '~/composables/useClientTheme'

const shareId = useRoute().params.shareId as string

useHead({
  title: 'Shared Controls',
  meta: [
    {
      name: 'description',
      content: 'Access shared controls remotely via the provided link.'
    }
  ]
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
          class="flex flex-col gap-2 min-w-[90vw] sm:min-w-[456px] md:min-w-[692px]"
          default-open
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
