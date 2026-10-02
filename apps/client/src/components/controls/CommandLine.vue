<script setup lang="ts">
import type { Client } from '@renderer/db/clients.db'
import type { Command } from '@renderer/db/commands.db'
import { formatCommandValue } from '@renderer/utils/commandLog'
import { computed } from 'vue'

// The log line under the Controls page's tiles: the latest command anyone sent, so the host can
// see at a glance what just happened and who did it.
const props = defineProps<{ latest: { command: Command; client?: Client } | null }>()

const time = computed(() =>
  props.latest ? new Date(props.latest.command.createdAt).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' }) : ''
)
</script>

<template>
  <div class="mx-5.5 mb-3.5 flex h-10 shrink-0 items-center gap-3.5 overflow-hidden rounded-field glass px-3.5 font-mono text-[11px] whitespace-nowrap text-muted">
    <template v-if="latest">
      <span class="shrink-0 text-success">IN ▸</span>
      <span class="min-w-0 truncate text-highlighted">{{ latest.command.controlName }}</span>
      <span class="shrink-0 text-primary">{{ formatCommandValue(latest.command.value) }}</span>
      <span class="min-w-0 truncate">by {{ latest.client?.displayName ?? 'unknown viewer' }}</span>
      <span class="ml-auto shrink-0">{{ time }}</span>
    </template>
    <span v-else>No viewer has used a control yet.</span>
  </div>
</template>
