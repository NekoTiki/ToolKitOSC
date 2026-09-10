<script setup lang="ts">
import { useBannedClientsDb } from '@renderer/composables/useBannedClientsDb'
import { formatUniqueKey, useClientsDb } from '@renderer/composables/useClientsDb'
import { useClientsListDrawer } from '@renderer/composables/useClientsListDrawer'
import type { Client } from '@renderer/db/clients.db'
import { db } from '@renderer/db/clients.db'
import { from, useObservable } from '@vueuse/rxjs'
import { liveQuery } from 'dexie'
import { computed } from 'vue'

const { open } = useClientsListDrawer()
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
  <!-- z-50: USlideover's own default has no explicit z-index (just DOM/portal order), which used
  to be enough since nothing else in this app had one either - now that AppHeader.vue's sticky bar
  does (z-10, needed to stay above the scrolling controls under it), an explicit positive z-index
  always wins over an implicit one regardless of DOM order, so without this the header rendered on
  top of the slideover. Same fix already applied to AreYouSureModal.vue/SettingsModal.vue. -->
  <USlideover
    v-model:open="open"
    inset
    title="Clients List"
    class="z-50 w-full max-w-xs"
    :ui="{ overlay: 'z-50' }"
  >
    <template #body>
      <div class="flex flex-col gap-4 overflow-y-auto">
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
      </div>
    </template>
  </USlideover>
</template>

<style scoped></style>
