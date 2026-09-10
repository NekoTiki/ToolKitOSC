<script setup lang="ts">
import type { OpenShockStatus } from '@renderer/composables/useOpenShock'
import { useOpenShock } from '@renderer/composables/useOpenShock'
import { computed } from 'vue'

// Not a persistent connection like Intiface (OpenShock is plain request/response HTTP), but the
// same "icon + hover popover" treatment still applies well: a glanceable status, with the details
// - here, the shockers found on the account - one hover away instead of only visible in Settings.
const { status, shockerNames } = useOpenShock()

const STATE_COLOR: Record<OpenShockStatus, string> = {
  valid: 'text-success',
  checking: 'text-warning',
  invalid: 'text-error',
  disabled: 'text-muted',
  unconfigured: 'text-muted'
}

const STATE_TEXT: Record<OpenShockStatus, string> = {
  valid: 'Connected',
  checking: 'Checking key…',
  invalid: 'Invalid key',
  disabled: 'Disabled',
  unconfigured: 'Not configured'
}

const shockerList = computed(() =>
  Array.from(shockerNames.value.entries()).map(([id, name]) => ({ id, name }))
)
</script>

<template>
  <UPopover
    mode="hover"
    :content="{ align: 'center', side: 'bottom' }"
  >
    <UIcon
      name="material-symbols:electric-bolt"
      class="size-5"
      :class="STATE_COLOR[status]"
    />

    <template #content>
      <div class="flex min-w-52 flex-col gap-2 p-3">
        <div
          class="flex items-center gap-1.5 text-sm font-medium"
          :class="STATE_COLOR[status]"
        >
          <UIcon
            name="material-symbols:electric-bolt"
            class="size-4"
          />
          OpenShock: {{ STATE_TEXT[status] }}
        </div>

        <template v-if="status === 'valid'">
          <div
            v-if="shockerList.length === 0"
            class="text-xs text-muted"
          >
            No shockers found on this account
          </div>
          <div
            v-else
            class="flex flex-col gap-1.5"
          >
            <div
              v-for="shocker in shockerList"
              :key="shocker.id"
              class="text-sm"
            >
              {{ shocker.name }}
            </div>
          </div>
        </template>
      </div>
    </template>
  </UPopover>
</template>

<style scoped></style>
