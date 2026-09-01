<script setup lang="ts">
import { useClientsDb } from '@renderer/composables/useClientsDb'
import type { Client } from '@renderer/db/clients.db'
import type { Command } from '@renderer/db/commands.db'
import { db } from '@renderer/db/commands.db'
import { from, useObservable } from '@vueuse/rxjs'
import { liveQuery } from 'dexie'

const props = defineProps<{ controlId: string }>()

const { getClients } = useClientsDb()

const commandsObservable = from(
  liveQuery<Command[]>(() =>
    db.commands
      .where('[controlId+createdAt]')
      .between([props.controlId, 0], [props.controlId, Infinity])
      .reverse()
      .limit(5)
      .toArray()
  )
)

const clientsObservable = from(getClients())

const commands = useObservable(commandsObservable, { initialValue: [] as Command[] })
const clients = useObservable(clientsObservable, { initialValue: [] as Client[] })
</script>

<template>
  <ClientDetails
    v-for="command in commands"
    :key="command.id"
    :client="
      clients.find(
        (client) => client.ip === command.ip && client.discordId === (command.discordId || null)
      )
    "
  >
    <span class="text-muted">
      {{ new Date(command.createdAt).toLocaleTimeString() }}
    </span>
  </ClientDetails>
</template>

<style scoped></style>
