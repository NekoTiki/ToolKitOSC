<script setup lang="ts">
import type { DropdownMenuItem } from '@nuxt/ui/components/DropdownMenu.vue'
import { useClientLogsModal } from '@renderer/composables/useClientLogsModal'
import type { Client } from '@renderer/db/clients.db'
import { db } from '@renderer/db/commands.db'
import { from, useObservable } from '@vueuse/rxjs'
import { liveQuery } from 'dexie'
import { ref } from 'vue'

const props = defineProps<{ client?: Omit<Client, 'createdAt' | 'uniqueKey'> }>()

const { openModal: openLogsModal } = useClientLogsModal()

const items = ref<DropdownMenuItem[]>([
  {
    label: 'See Logs',
    icon: 'i-lucide-history',
    onSelect: () => {
      if (props.client) openLogsModal(props.client)
    }
  },
  { type: 'separator' },
  {
    label: 'Ban',
    icon: 'i-lucide-ban',
    color: 'error',
    children: [
      {
        label: 'Ban by IP',
        icon: 'iconoir:ip-address-tag',
        color: 'error'
      },
      {
        disabled: !props.client?.discordId,
        label: 'Ban by Discord ID',
        icon: 'ic:baseline-discord',
        color: 'error'
      }
    ]
  }
])

const commandsByIpObservable = from(
  liveQuery<number>(() =>
    db.commands
      .where('ip')
      .equals(props.client?.ip || '')
      .reverse()
      .count()
  )
)

const commandsByDiscordIdObservable = from(
  liveQuery<number>(() =>
    db.commands
      .where('discordId')
      .equals(props.client?.discordId || '')
      .reverse()
      .count()
  )
)

const ipCommands = useObservable(commandsByIpObservable, { initialValue: 0 })
const discordCommands = useObservable(commandsByDiscordIdObservable, { initialValue: 0 })

const formatNumber = (num: number): string => num.toLocaleString('en-US')
</script>

<template>
  <div
    v-if="client"
    class="flex w-full items-center gap-2"
  >
    <UAvatar :src="client.avatar" />
    <div class="flex grow items-center justify-between gap-1">
      <div class="flex items-center gap-1">
        <span>{{ client.displayName }}</span>
        <UPopover
          mode="hover"
          :content="{ align: 'center', side: 'bottom', sideOffset: 8 }"
        >
          <UIcon
            name="iconoir:ip-address-tag"
            class="text-primary"
          />

          <template #content>
            <UCard
              class="text-primary"
              :ui="{ body: 'p-2 sm:p-2' }"
            >
              ({{ client.ip }})
            </UCard>
          </template>
        </UPopover>
        <slot>
          <span
            v-if="discordCommands"
            class="flex items-center text-secondary"
          >
            <UIcon name="lucide:download" />
            <span class="ml-1">({{ formatNumber(discordCommands) }})</span>
          </span>
          <span
            v-else
            class="flex items-center text-secondary"
          >
            <UIcon name="lucide:download" />
            <span class="ml-1">({{ formatNumber(ipCommands) }})</span>
          </span>
        </slot>
      </div>
      <UDropdownMenu
        :items="items"
        :content="{ align: 'end', side: 'bottom', sideOffset: 8 }"
        :ui="{ content: 'w-48' }"
        class="justify-self-end"
      >
        <UButton
          icon="i-lucide-menu"
          color="neutral"
          variant="outline"
          size="sm"
        />
      </UDropdownMenu>
    </div>
  </div>
</template>

<style scoped></style>
