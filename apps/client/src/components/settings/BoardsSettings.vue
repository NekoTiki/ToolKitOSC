<script setup lang="ts">
import type { StatusInfo } from '@renderer/components/settings/StatusLine.vue'
import StatusLine from '@renderer/components/settings/StatusLine.vue'
import FormSection from '@renderer/components/ui/FormSection.vue'
import { useAreYouSureModal } from '@renderer/composables/useAreYouSureModal'
import { useBoards } from '@renderer/composables/useBoards'
import type { Board, BoardOutputPin } from '@renderer/lib/tauri-bridge'
import { timeAgo } from '@renderer/utils/time'
import { useNow } from '@vueuse/core'
import { computed, ref } from 'vue'

const {
  boards,
  found,
  pairing,
  pairingDeadline,
  pairingActive,
  add,
  addByAddress,
  cancelPairing,
  dismissPairing,
  remove
} = useBoards()
const { openModal: confirm } = useAreYouSureModal()

const now = useNow({ interval: 1000 })
const address = ref('')

const secondsLeft = computed(() =>
  pairingDeadline.value
    ? Math.max(0, Math.ceil((pairingDeadline.value - now.value.getTime()) / 1000))
    : null
)

const statusInfo = (board: Board): StatusInfo => {
  switch (board.status) {
    case 'online':
      return { icon: 'i-lucide-circle-check', class: 'text-success', label: 'Online' }
    case 'offline':
      return {
        icon: 'i-lucide-circle-x',
        class: 'text-error',
        label: board.offlineSince
          ? `Offline since ${timeAgo(board.offlineSince, now.value)}`
          : 'Offline'
      }
    case 'needs-pairing':
      return { icon: 'i-lucide-link-2-off', class: 'text-warning', label: 'Needs pairing again' }
    case 'update-firmware':
      return {
        icon: 'i-lucide-triangle-alert',
        class: 'text-warning',
        label: 'Update the firmware'
      }
    default:
      return {
        icon: 'i-lucide-circle-dashed',
        class: 'text-muted animate-pulse',
        label: 'Connecting…'
      }
  }
}

const details = (board: Board): string =>
  [`${board.host}`, board.fw && `firmware ${board.fw}`, board.chip, board.board]
    .filter(Boolean)
    .join(' · ')

const pinLabel = (output: BoardOutputPin): string =>
  output.label ? `GPIO ${output.pin} · ${output.label}` : `GPIO ${output.pin}`

const removeBoard = async (board: Board): Promise<void> => {
  const ok = await confirm({
    title: `Remove ${board.name}?`,
    message:
      board.status === 'online'
        ? 'The board forgets this PC. To add it again, you will have to press its BOOT button.'
        : "The board is offline, so it can't be told. To add it to another PC, unpair it on the board: hold BOOT for 3 seconds.",
    confirmText: 'Remove'
  })

  if (ok) await remove(board.id)
}

// Pairing again a board that refused its token: same as adding it, through its last address.
const repair = (board: Board): Promise<void> =>
  found.value.some((f) => f.id === board.id)
    ? add(board.id)
    : addByAddress(`${board.host}:${board.port}`)

const submitAddress = async (): Promise<void> => {
  if (!address.value.trim()) return
  await addByAddress(address.value)
  if (pairing.value?.state === 'paired') address.value = ''
}
</script>

<template>
  <UAlert
    v-if="pairing?.state === 'connecting'"
    color="neutral"
    variant="subtle"
    icon="i-lucide-loader-circle"
    :title="`Connecting to ${pairing.name}…`"
    :actions="[{ label: 'Cancel', color: 'neutral', variant: 'outline', onClick: cancelPairing }]"
    :ui="{ icon: 'animate-spin' }"
  />
  <UAlert
    v-else-if="pairing?.state === 'waiting'"
    color="primary"
    variant="subtle"
    icon="i-lucide-pointer"
    :title="`Press the BOOT button on ${pairing.name}`"
    :description="`Its LED blinks blue while it waits. ${secondsLeft ?? ''} s left.`"
    :actions="[{ label: 'Cancel', color: 'neutral', variant: 'outline', onClick: cancelPairing }]"
  />
  <UAlert
    v-else-if="pairing?.state === 'paired'"
    color="success"
    variant="subtle"
    icon="i-lucide-circle-check"
    :title="`${pairing.name} added`"
    close
    @update:open="dismissPairing"
  />
  <UAlert
    v-else-if="pairing?.state === 'failed'"
    color="error"
    variant="subtle"
    icon="i-lucide-circle-x"
    title="Couldn't add the board"
    :description="pairing.reason"
    close
    @update:open="dismissPairing"
  />

  <FormSection
    title="Your boards"
    :description="boards.length ? undefined : 'Boards you add show up here.'"
  >
    <div
      v-if="boards.length"
      class="grid divide-y divide-(--ui-border)"
    >
      <div
        v-for="board in boards"
        :key="board.id"
        class="grid gap-2.5 py-3 first:pt-0 last:pb-0"
      >
        <div class="flex items-center gap-3">
          <span
            class="grid size-10 shrink-0 place-items-center rounded-md bg-(--aurora-well) text-muted"
          >
            <UIcon
              name="i-lucide-cpu"
              class="size-5"
            />
          </span>
          <div class="grid min-w-0 flex-1 gap-0.5">
            <b class="truncate font-medium text-highlighted">{{ board.name }}</b>
            <small class="truncate text-xs text-muted">{{ details(board) }}</small>
            <StatusLine :status="statusInfo(board)" />
          </div>
          <UButton
            v-if="board.status === 'needs-pairing'"
            color="primary"
            variant="subtle"
            :disabled="pairingActive"
            @click="repair(board)"
          >
            Pair again
          </UButton>
          <UButton
            icon="i-lucide-trash-2"
            color="neutral"
            variant="subtle"
            @click="removeBoard(board)"
          >
            Remove
          </UButton>
        </div>
        <div
          v-if="board.outputs.length"
          class="flex flex-wrap gap-1.5 pl-13"
        >
          <span
            v-for="output in board.outputs"
            :key="output.pin"
            class="rounded-full bg-(--aurora-well) px-2.5 py-0.5 font-mono text-xs text-muted"
          >{{ pinLabel(output) }}</span>
        </div>
      </div>
    </div>
  </FormSection>

  <FormSection
    title="Found on your network"
    :description="
      found.length
        ? undefined
        : 'Looking for boards… Make sure the board is on the same Wi-Fi as this PC.'
    "
  >
    <div
      v-if="found.length"
      class="grid divide-y divide-(--ui-border)"
    >
      <div
        v-for="board in found"
        :key="board.id"
        class="flex min-h-13 items-center gap-3"
      >
        <span
          class="grid size-10 shrink-0 place-items-center rounded-md bg-(--aurora-well) text-muted"
        >
          <UIcon
            name="i-lucide-cpu"
            class="size-5"
          />
        </span>
        <div class="grid min-w-0 flex-1 gap-0.5">
          <b class="truncate font-medium text-highlighted">{{ board.name }}</b>
          <small class="truncate text-xs text-muted">
            tkosc-{{ board.id }}.local · {{ board.paired ? 'Added on another PC' : 'Ready to add' }}
          </small>
        </div>
        <UButton
          icon="i-lucide-plus"
          :disabled="pairingActive"
          @click="add(board.id)"
        >
          Add
        </UButton>
      </div>
    </div>
  </FormSection>

  <FormSection
    title="Add by address"
    description="For networks where boards don't show up on their own, like VPNs. The board's serial monitor prints its address."
  >
    <div class="flex gap-2">
      <UInput
        v-model="address"
        placeholder="192.168.1.40 or tkosc-a1b2c3.local"
        class="w-full"
        autocomplete="off"
        :ui="{ base: 'font-mono' }"
        @keyup.enter="submitAddress"
      />
      <UButton
        icon="i-lucide-plus"
        :disabled="pairingActive || !address.trim()"
        @click="submitAddress"
      >
        Add
      </UButton>
    </div>
  </FormSection>
</template>
