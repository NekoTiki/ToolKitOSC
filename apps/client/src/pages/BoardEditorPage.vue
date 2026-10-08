<script setup lang="ts">
import StatusLine from '@renderer/components/settings/StatusLine.vue'
import FormSection from '@renderer/components/ui/FormSection.vue'
import { useAreYouSureModal } from '@renderer/composables/useAreYouSureModal'
import { boardStatusInfo, pinName, useBoards } from '@renderer/composables/useBoards'
import { useStopEverything } from '@renderer/composables/useStopEverything'
import type { BoardOutputConfig } from '@renderer/lib/tauri-bridge'
import { useNow } from '@vueuse/core'
import { computed, ref, watch } from 'vue'
import { onBeforeRouteLeave, useRoute } from 'vue-router'

// One ESP32 board's outputs ("Fan on GPIO 4, max 10 s"), edited as a list and sent to the board on
// Save. Reached from Settings › ESP32 boards › Edit.
const route = useRoute()
const toast = useToast()
const { boards, lastError, saveOutputs, testOutput } = useBoards()
const { paused } = useStopEverything()
const { openModal: confirm } = useAreYouSureModal()
const now = useNow({ interval: 1000 })

const boardId = String(route.params.boardId)
const board = computed(() => boards.value.find((b) => b.id === boardId))

const DEFAULT_MAX_ON_SECONDS = 10

interface Row {
  key: number
  pin: number | undefined
  label: string
  activeLow: boolean
  maxOnSeconds: number
}

let nextKey = 0
const toRow = (output: BoardOutputConfig): Row => ({
  key: nextKey++,
  pin: output.pin,
  label: output.label,
  activeLow: output.activeLow,
  maxOnSeconds: output.maxOnMs / 1000
})
const toOutput = (row: Row): BoardOutputConfig => ({
  pin: row.pin ?? -1,
  label: row.label.trim(),
  activeLow: row.activeLow,
  maxOnMs: Math.round(row.maxOnSeconds * 1000)
})

const rows = ref<Row[]>([])
const saving = ref(false)
// What's on the board, to tell what changed.
const savedJson = ref('[]')

const reset = (): void => {
  const config = board.value?.config ?? []
  rows.value = config.map(toRow)
  savedJson.value = JSON.stringify(config)
}

// Once the board has loaded (the list arrives asynchronously on a fresh start).
watch(() => board.value?.id, reset, { immediate: true })

const dirty = computed(() => JSON.stringify(rows.value.map(toOutput)) !== savedJson.value)

const rowError = (row: Row): string | null => {
  if (!row.label.trim()) return 'Give it a name.'
  if (row.pin === undefined) return 'Pick a pin.'
  if (!(row.maxOnSeconds >= 0.1 && row.maxOnSeconds <= 3600)) return 'Max on-time: 0.1 s to 1 hour.'
  return null
}
const valid = computed(() => rows.value.every((row) => !rowError(row)))

// Pins offered by the board that no other row uses.
const pinItems = (row: Row): { label: string; value: number }[] =>
  (board.value?.outputs ?? [])
    .filter((p) => p.pin === row.pin || !rows.value.some((r) => r.pin === p.pin))
    .map((p) => ({ label: pinName(p.pin, p.label), value: p.pin }))

// The Board card's label/value list.
const details = computed((): { label: string; value: string; mono?: boolean }[] =>
  board.value
    ? [
        { label: 'Address', value: `${board.value.host}:${board.value.port}`, mono: true },
        { label: 'Firmware', value: board.value.fw ?? 'Unknown' },
        { label: 'Chip', value: board.value.chip ?? 'Unknown' },
        { label: 'Board profile', value: board.value.board ?? 'Unknown' }
      ]
    : []
)

const freePins = computed(() => (board.value?.outputs ?? []).filter((p) => !rows.value.some((r) => r.pin === p.pin)))

const addRow = (): void => {
  rows.value.push({
    key: nextKey++,
    pin: freePins.value[0]?.pin,
    label: '',
    activeLow: false,
    maxOnSeconds: DEFAULT_MAX_ON_SECONDS
  })
}

const removeRow = (row: Row): void => {
  rows.value = rows.value.filter((r) => r.key !== row.key)
}

// Test sends to what's on the board, so only saved, unchanged outputs can be tested.
const testBlocked = (row: Row): string | null => {
  if (paused.value) return 'Stop everything is on'
  if (board.value?.status !== 'online') return 'Board offline'
  const saved = board.value.config?.find((o) => o.pin === row.pin)
  if (!saved || JSON.stringify(saved) !== JSON.stringify(toOutput(row))) return 'Save to test'
  return null
}

const test = async (row: Row): Promise<void> => {
  if (row.pin === undefined) return

  try {
    await testOutput(boardId, row.pin)
  } catch (error) {
    toast.add({ title: String(error), icon: 'i-lucide-circle-x', color: 'error' })
  }
}

const save = async (): Promise<void> => {
  if (!valid.value || saving.value) return
  saving.value = true

  try {
    const outputs = rows.value.map(toOutput)
    await saveOutputs(boardId, outputs)
    savedJson.value = JSON.stringify(outputs)
    toast.add({
      title: 'Outputs saved',
      description: board.value?.status === 'online' ? undefined : 'They reach the board when it comes back online.',
      icon: 'i-lucide-check',
      color: 'success'
    })
  } catch (error) {
    toast.add({ title: "Couldn't save", description: String(error), icon: 'i-lucide-circle-x', color: 'error' })
  } finally {
    saving.value = false
  }
}

const outputName = (pin: number | null): string =>
  board.value?.config?.find((o) => o.pin === pin)?.label ?? (pin === null ? 'An output' : `GPIO ${pin}`)

watch(lastError, (error) => {
  if (!error || error.id !== boardId) return

  const description =
    error.code === 'max-on'
      ? `${outputName(error.pin)} reached its max on-time. It stays off until you save its outputs again or use Stop everything.`
      : error.code === 'bad-pin'
        ? `The board doesn't have ${outputName(error.pin)} set up. Save the outputs again.`
        : `The board answered "${error.code}".`
  toast.add({ title: `${board.value?.name ?? 'The board'} refused`, description, icon: 'i-lucide-circle-x', color: 'error' })
  lastError.value = null
})

onBeforeRouteLeave(async () => {
  if (!dirty.value) return true

  return confirm({ title: 'Discard changes?', message: "Your changes to this board's outputs will be lost.", confirmText: 'Discard' })
})
</script>

<template>
  <PageLayout>
    <template #header>
      <PageHeader
        :title="board?.name ?? 'Board not found'"
        :crumbs="[
          { label: 'Settings', to: '/settings' },
          { label: 'ESP32 boards', to: '/settings/esp32' },
          { label: board?.name ?? boardId }
        ]"
      />
    </template>

    <div
      v-if="!board"
      class="grid max-w-3xl gap-3.5"
    >
      <FormSection
        title="This board isn't added any more"
        description="It may have been removed."
      >
        <div>
          <UButton
            color="neutral"
            variant="subtle"
            to="/settings/esp32"
          >
            Back to ESP32 boards
          </UButton>
        </div>
      </FormSection>
    </div>

    <!-- Side by side once there's room: the board's details in a narrow column, its outputs in the rest. -->
    <div
      v-else
      class="grid max-w-6xl items-start gap-3.5 lg:grid-cols-[18rem_minmax(0,1fr)]"
    >
      <FormSection
        title="Board"
        class="lg:sticky lg:top-0"
      >
        <StatusLine :status="boardStatusInfo(board, now)" />
        <!-- Label above value, so it fits the narrow column. -->
        <dl class="grid gap-2.5 text-sm">
          <div
            v-for="item in details"
            :key="item.label"
            class="grid min-w-0 gap-0.5"
          >
            <dt class="text-xs text-muted">
              {{ item.label }}
            </dt>
            <dd
              class="truncate"
              :class="{ 'font-mono': item.mono }"
              :title="item.value"
            >
              {{ item.value }}
            </dd>
          </div>
        </dl>
      </FormSection>

      <FormSection
        title="Outputs"
        :description="`What's wired to the board. ${board.outputs.length} pins available. Test switches an output on for half a second.`"
      >
        <template #actions>
          <UButton
            icon="i-lucide-plus"
            :disabled="!freePins.length"
            @click="addRow"
          >
            Add output
          </UButton>
        </template>

        <p
          v-if="!rows.length"
          class="text-sm text-muted"
        >
          No outputs yet. Add one for each thing wired to the board, like a fan or an LED strip.
        </p>

        <div
          v-for="row in rows"
          :key="row.key"
          class="grid gap-3 rounded-field bg-(--aurora-well) p-3.5"
        >
          <div class="grid gap-3 sm:grid-cols-[minmax(0,1fr)_14rem]">
            <UFormField label="Name">
              <UInput
                v-model="row.label"
                placeholder="Fan"
                class="w-full"
                autocomplete="off"
              />
            </UFormField>
            <UFormField label="Pin">
              <USelect
                v-model="row.pin"
                :items="pinItems(row)"
                placeholder="Pick a pin"
                class="w-full"
              />
            </UFormField>
          </div>

          <div class="flex flex-wrap items-end gap-3">
            <UFormField
              label="Max on-time"
              description="Seconds. The board never keeps it on longer in one go."
              class="min-w-56 flex-1"
            >
              <UInput
                v-model.number="row.maxOnSeconds"
                type="number"
                min="0.1"
                max="3600"
                step="0.5"
                class="w-full"
              />
            </UFormField>
            <USwitch
              v-model="row.activeLow"
              label="Active low"
              description="For relay modules that switch on when the pin is LOW."
              class="flex-1"
            />
          </div>

          <div class="flex flex-wrap items-center gap-2">
            <p
              v-if="rowError(row)"
              class="mr-auto text-[13px] text-error"
            >
              {{ rowError(row) }}
            </p>
            <p
              v-else-if="testBlocked(row)"
              class="mr-auto text-[13px] text-muted"
            >
              {{ testBlocked(row) }}
            </p>
            <span
              v-else
              class="mr-auto"
            />
            <UButton
              icon="i-lucide-zap"
              color="neutral"
              variant="subtle"
              :disabled="!!testBlocked(row) || !!rowError(row)"
              @click="test(row)"
            >
              Test
            </UButton>
            <UButton
              icon="i-lucide-trash-2"
              color="neutral"
              variant="subtle"
              @click="removeRow(row)"
            >
              Remove
            </UButton>
          </div>
        </div>
      </FormSection>
    </div>

    <template
      v-if="board"
      #footer
    >
      <p class="mr-auto flex min-w-0 items-center gap-1.5 text-[12.5px] text-muted">
        <UIcon
          name="i-lucide-info"
          class="size-4 shrink-0"
        />
        Saving sends the outputs to the board and switches them off for a moment.
      </p>
      <UButton
        color="neutral"
        variant="subtle"
        :disabled="!dirty"
        @click="reset"
      >
        Undo changes
      </UButton>
      <UButton
        icon="i-lucide-check"
        :loading="saving"
        :disabled="!dirty || !valid"
        @click="save"
      >
        Save changes
      </UButton>
    </template>
  </PageLayout>
</template>
