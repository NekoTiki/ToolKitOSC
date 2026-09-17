<script setup lang="ts">
import { useBannedClientsDb } from '@renderer/composables/useBannedClientsDb'
import { formatUniqueKey, useClientsDb } from '@renderer/composables/useClientsDb'
import type { Client } from '@renderer/db/clients.db'
import { db } from '@renderer/db/clients.db'
import { from, useObservable } from '@vueuse/rxjs'
import { liveQuery } from 'dexie'
import { computed } from 'vue'

const open = defineModel<boolean>('open')
const { onlineClients } = useClientsDb()
const { isBanned } = useBannedClientsDb()
const clientsObservable = from(liveQuery<Client[]>(() => db.clients.toArray()))

const clientList = useObservable(clientsObservable, { initialValue: [] as Client[] })

const bannedClientsList = computed(() =>
  clientList.value.filter((client) => isBanned(client.ip, client.discordId))
)
const onlineClientsList = computed(() =>
  clientList.value.filter(
    (client) =>
      !isBanned(client.ip, client.discordId) &&
      onlineClients.value.includes(formatUniqueKey(client.ip, client.discordId))
  )
)
const offlineClientsList = computed(() =>
  clientList.value.filter(
    (client) =>
      !isBanned(client.ip, client.discordId) &&
      !onlineClients.value.includes(formatUniqueKey(client.ip, client.discordId))
  )
)
</script>

<template>
  <USlideover
    v-model:open="open"
    inset
    title="Clients List"
    class="w-full max-w-xs"
    :ui="{ body: 'overflow-hidden' }"
  >
    <template #body>
      <!-- Three separately-headed sub-lists, not one flat array, so this uses UScrollArea's plain
      default-slot mode rather than its `:items` prop - `overflow-hidden` on the slideover's own
      body (above) cancels its default `overflow-y-auto` so this is the only scroll region. -->
      <UScrollArea
        class="h-full"
        :ui="{ viewport: 'gap-4' }"
      >
        <div
          v-if="onlineClientsList.length"
          class="flex flex-col gap-2"
        >
          <div class="flex items-center gap-2">
            <UIcon name="ion:cloud" />
            <h3 class="font-semibold">
              Online Clients
            </h3>
          </div>
          <ClientDetails
            v-for="client in onlineClientsList"
            :key="client.id"
            :client="client"
          />
        </div>

        <div
          v-if="offlineClientsList.length"
          class="flex flex-col gap-2"
        >
          <div class="flex items-center gap-2">
            <UIcon name="ion:cloud-offline" />
            <h3 class="font-semibold">
              Offline Clients
            </h3>
          </div>
          <ClientDetails
            v-for="client in offlineClientsList"
            :key="client.id"
            :client="client"
          />
        </div>

        <div
          v-if="bannedClientsList.length"
          class="flex flex-col gap-2"
        >
          <div class="flex items-center gap-2 text-error">
            <UIcon name="i-lucide-ban" />
            <h3 class="font-semibold">
              Banned Clients
            </h3>
          </div>
          <ClientDetails
            v-for="client in bannedClientsList"
            :key="client.id"
            :client="client"
          />
        </div>
      </UScrollArea>
    </template>
  </USlideover>
</template>

<style scoped></style>
