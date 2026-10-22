<script setup lang="ts">
import FormSection from '@renderer/components/ui/FormSection.vue'
import SegmentedControl from '@renderer/components/ui/SegmentedControl.vue'
import { useAreYouSureModal } from '@renderer/composables/useAreYouSureModal'
import { useMcpClients } from '@renderer/composables/useMcpBridge'
import type { McpLevels, McpPermission, McpSettings } from '@renderer/lib/tauri-bridge'
import { api } from '@renderer/lib/tauri-bridge'
import { timeAgo } from '@renderer/utils/time'
import { computed, ref } from 'vue'

// Settings > Assistants: the local MCP server (src-tauri/src/mcp) that Claude Code, Cursor and
// other AI assistants on this PC connect to. Turned on and off on the Add-ons page; here you set
// what assistants may do and copy the setup for a client. The key stays in Rust until you ask to
// see or copy it.
const toast = useToast()
const { openModal } = useAreYouSureModal()

const settings = ref<McpSettings | null>(null)
const clients = useMcpClients()
const key = ref<string | null>(null)

void api.mcpSettings().then((value) => (settings.value = value))

const PERMISSIONS: { value: McpPermission; label: string }[] = [
  { value: 'off', label: 'Off' },
  { value: 'ask', label: 'Ask' },
  { value: 'on', label: 'On' }
]

const LEVELS: { key: keyof McpLevels; label: string; description: string }[] = [
  { key: 'read', label: 'Read', description: 'Your avatar, controls, presets and Activity' },
  {
    key: 'play',
    label: 'Play',
    description: 'Controls, presets, toys and the chatbox, like a viewer'
  },
  {
    key: 'edit',
    label: 'Edit',
    description: 'Drafts of controls, profiles, automations and patterns'
  },
  { key: 'shock', label: 'Shock', description: 'OpenShock controls, with their usual limits' }
]

type Client = 'claude-code' | 'claude-desktop' | 'cursor' | 'vscode'

const client = ref<Client>('claude-code')

const CLIENTS: { value: Client; label: string }[] = [
  { value: 'claude-code', label: 'Claude Code' },
  { value: 'claude-desktop', label: 'Claude Desktop' },
  { value: 'cursor', label: 'Cursor' },
  { value: 'vscode', label: 'VS Code' }
]

// Claude Desktop starts this app with --mcp and talks to it over stdio; that reads the key itself.
const command = ref('')

void api.mcpCommand().then((value) => (command.value = value))

const setup = (withKey: string): string => {
  const url = `http://127.0.0.1:${settings.value?.port}/mcp`
  const headers = { Authorization: `Bearer ${withKey}` }

  switch (client.value) {
    case 'claude-code':
      return `claude mcp add --transport http toolkitosc ${url} --header "Authorization: Bearer ${withKey}"`
    case 'claude-desktop':
      return JSON.stringify({ mcpServers: { toolkitosc: { command: command.value, args: ['--mcp'] } } }, null, 2)
    case 'cursor':
      return JSON.stringify({ mcpServers: { toolkitosc: { url, headers } } }, null, 2)
    case 'vscode':
      return JSON.stringify({ servers: { toolkitosc: { type: 'http', url, headers } } }, null, 2)
  }
}

const HOW: Record<Client, string> = {
  'claude-code': 'Run this in a terminal.',
  'claude-desktop': 'In Claude Desktop, open Settings > Developer > Edit Config and add this to claude_desktop_config.json. It needs no key.',
  cursor: 'Add this to ~/.cursor/mcp.json.',
  vscode: 'Add this to .vscode/mcp.json, or run MCP: Add Server.'
}

const shown = computed(() => setup(key.value ?? '<key>'))

const toggleKey = async (): Promise<void> => {
  try {
    key.value = key.value ? null : await api.mcpKey()
  } catch (error) {
    toast.add({ title: 'Could not read the key', description: String(error), color: 'error' })
  }
}

const copy = async (): Promise<void> => {
  try {
    await navigator.clipboard.writeText(setup(key.value ?? (await api.mcpKey())))
    toast.add({
      title: client.value === 'claude-desktop' ? 'Copied' : 'Copied, with your key',
      icon: 'i-lucide-clipboard-check',
      color: 'success'
    })
  } catch (error) {
    toast.add({ title: 'Could not copy it', description: String(error), color: 'error' })
  }
}

const newKey = async (): Promise<void> => {
  const confirmed = await openModal({
    title: 'Make a new key?',
    message: 'Every assistant set up with the current key is cut off until you set it up again.',
    confirmText: 'New key'
  })
  if (!confirmed) return
  try {
    const value = await api.mcpNewKey()
    if (key.value) key.value = value
    toast.add({
      title: 'New key made',
      description: 'Copy the setup again for each assistant.',
      color: 'success'
    })
  } catch (error) {
    toast.add({ title: 'Could not make a new key', description: String(error), color: 'error' })
  }
}

const save = (edit: (settings: McpSettings) => void): void => {
  if (!settings.value) return
  edit(settings.value)
  void api
    .mcpSave(JSON.parse(JSON.stringify(settings.value)))
    .catch((error) =>
      toast.add({
        title: 'Could not save the assistant settings',
        description: String(error),
        color: 'error'
      })
    )
}

const port = ref<number | null>(null)

const savePort = (): void => {
  const value = Number(port.value)
  if (!Number.isInteger(value) || value < 1024 || value > 65535 || value === settings.value?.port)
    return
  save((s) => (s.port = value))
  port.value = null
}
</script>

<template>
  <template v-if="settings">
    <FormSection
      title="Connected"
      :description="clients.length ? undefined : 'No assistant connected. Set one up below.'"
    >
      <div
        v-if="clients.length"
        class="grid divide-y divide-(--ui-border)"
      >
        <div
          v-for="item in clients"
          :key="item.id"
          class="flex min-h-13 items-center gap-3"
        >
          <span
            class="size-2.5 shrink-0 rounded-full bg-success shadow-[0_0_8px_var(--ui-success)]"
          />
          <div class="grid min-w-0 flex-1">
            <b class="truncate font-medium text-highlighted">{{ item.name }}</b>
            <small class="truncate text-muted">
              Since {{ timeAgo(item.since) }} · {{ item.calls }}
              {{ item.calls === 1 ? 'call' : 'calls'
              }}<template v-if="item.lastTool"> · last: {{ item.lastTool }}</template>
            </small>
          </div>
        </div>
      </div>
    </FormSection>

    <FormSection
      title="What assistants may do"
      description="Ask makes you allow each call. Stop everything always works for them, and only you can resume."
    >
      <div class="grid gap-3 sm:grid-cols-2">
        <div
          v-for="level in LEVELS"
          :key="level.key"
          class="grid content-start gap-2 rounded-field bg-(--aurora-well) p-3"
        >
          <div class="grid gap-0.5">
            <b class="font-semibold text-highlighted">{{ level.label }}</b>
            <small class="text-muted">{{ level.description }}</small>
          </div>
          <SegmentedControl
            :model-value="settings.levels[level.key]"
            :items="PERMISSIONS"
            :aria-label="`${level.label} permission`"
            accent
            @update:model-value="save((s) => (s.levels[level.key] = $event))"
          />
        </div>
      </div>
      <USwitch
        :model-value="settings.rawParameters"
        label="Raw parameters"
        description="Play may set any avatar parameter directly, except ones a control locked for assistants drives."
        @update:model-value="save((s) => (s.rawParameters = !!$event))"
      />
      <USwitch
        :model-value="settings.switchProfiles"
        label="Switch profiles"
        description="Play may change the active profile, which decides what everyone may do."
        @update:model-value="save((s) => (s.switchProfiles = !!$event))"
      />
    </FormSection>

    <FormSection
      title="Connect an assistant"
      description="It only works on this PC, while ToolKitOSC runs."
    >
      <SegmentedControl
        v-model="client"
        :items="CLIENTS"
        aria-label="Assistant"
      />
      <p class="text-sm text-muted">
        {{ HOW[client] }}
      </p>
      <pre
        class="overflow-x-auto rounded-2xl bg-(--aurora-well) p-3 font-mono text-xs whitespace-pre text-default"
      >{{ shown }}</pre>
      <div class="flex flex-wrap items-center gap-2">
        <UButton
          label="Copy"
          icon="i-lucide-clipboard-copy"
          @click="copy"
        />
        <UButton
          :label="key ? 'Hide key' : 'Show key'"
          :icon="key ? 'i-lucide-eye-off' : 'i-lucide-eye'"
          color="neutral"
          variant="outline"
          @click="toggleKey"
        />
        <UButton
          label="New key"
          icon="i-lucide-key-round"
          color="neutral"
          variant="outline"
          @click="newKey"
        />
      </div>
      <UFormField
        label="Port"
        description="Change it only if another program uses it, then set up your assistants again."
      >
        <UInput
          v-model.number="port"
          type="number"
          :placeholder="String(settings.port)"
          class="w-32"
          @change="savePort"
          @keyup.enter="savePort"
        />
      </UFormField>
    </FormSection>
  </template>
</template>
