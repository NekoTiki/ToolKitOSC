<script setup lang="ts">
import type { DropdownMenuItem } from '@nuxt/ui/components/DropdownMenu.vue'
import { useAreYouSureModal } from '@renderer/composables/useAreYouSureModal'
import { useBannedClientsDb } from '@renderer/composables/useBannedClientsDb'
import { useClientLogsModal } from '@renderer/composables/useClientLogsModal'
import type { Client } from '@renderer/db/clients.db'
import { db } from '@renderer/db/commands.db'
import { from, useObservable } from '@vueuse/rxjs'
import { liveQuery } from 'dexie'
import { computed } from 'vue'

const props = defineProps<{ client?: Omit<Client, 'createdAt' | 'uniqueKey'> }>()

const { openModal: openLogsModal } = useClientLogsModal()
const { openModal: openAreYouSure } = useAreYouSureModal()
const { ban, unban, isBanned } = useBannedClientsDb()

const banStatus = computed(() =>
  props.client ? isBanned(props.client.ip, props.client.discordId) : undefined
)

const banByIp = async (): Promise<void> => {
  if (!props.client) return

  const confirmed = await openAreYouSure({
    title: 'Ban by IP',
    message: `This will ban **${props.client.displayName}**'s **IP address** (\`${props.client.ip}\`). Anyone connecting from this IP will no longer be able to use controls, until unbanned.`,
    confirmText: 'Ban'
  })

  if (confirmed) ban('ip', props.client.ip)
}

const banByDiscord = async (): Promise<void> => {
  if (!props.client?.discordId) return

  const confirmed = await openAreYouSure({
    title: 'Ban by Discord ID',
    message: `This will ban **${props.client.displayName}**'s **Discord account**. They will no longer be able to use controls from any IP, until unbanned.`,
    confirmText: 'Ban'
  })

  if (confirmed) ban('discord', props.client.discordId)
}

const handleUnban = async (): Promise<void> => {
  if (banStatus.value?.id === undefined) return

  const modeLabel = banStatus.value.scope === 'discord' ? 'Discord account' : 'IP address'

  const confirmed = await openAreYouSure({
    title: 'Unban',
    message: `This will unban **${props.client?.displayName}**'s **${modeLabel}**. They will be able to use controls again.`,
    confirmText: 'Unban'
  })

  if (confirmed) unban(banStatus.value.id)
}

const items = computed<DropdownMenuItem[]>(() => [
  {
    label: 'See Logs',
    icon: 'i-lucide-history',
    onSelect: () => {
      if (props.client) openLogsModal(props.client)
    }
  },
  { type: 'separator' },
  banStatus.value
    ? {
        label: 'Unban',
        icon: 'i-lucide-shield-check',
        color: 'success',
        onSelect: handleUnban
      }
    : {
        label: 'Ban',
        icon: 'i-lucide-ban',
        color: 'error',
        children: [
          {
            label: 'IP',
            icon: 'iconoir:ip-address-tag',
            color: 'error',
            onSelect: banByIp
          },
          ...(props.client?.discordId
            ? [
                {
                  label: 'Discord ID',
                  icon: 'ic:baseline-discord',
                  color: 'error' as const,
                  onSelect: banByDiscord
                }
              ]
            : [])
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
        <span :class="{ 'text-error': banStatus }">{{ client.displayName }}</span>
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
