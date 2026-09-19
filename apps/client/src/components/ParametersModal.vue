<script setup lang="ts">
import AddressListToggle from '@renderer/components/control-modal/AddressListToggle.vue'
import { useAvatarDetails } from '@renderer/composables/useAvatarDetails'
import { useOscMessages } from '@renderer/composables/useOscMessages'
import type { OSCArg } from '@renderer/env'
import { api } from '@renderer/lib/tauri-bridge'
import { isNoisyAddress } from '@renderer/utils/addressFilter'
import { computed, onUnmounted, ref, useTemplateRef, watch } from 'vue'

const open = defineModel<boolean>('open')

const { avatarDetails } = useAvatarDetails()
const { args } = useOscMessages()
const toast = useToast()

type ParameterRow = {
  name: string
  address: string
  kind: string | undefined
  received: boolean
  value: OSCArg | undefined
}

const showAll = ref(false)
const search = ref('')
const sortDesc = ref(false)
const pulling = ref(false)

const sortIcon = computed(() =>
  sortDesc.value ? 'i-lucide-arrow-down-wide-narrow' : 'i-lucide-arrow-up-narrow-wide'
)

// output.address is the one VRChat actually pushes updates from; input.address is only where we'd
// send a write to. Falls back to input for the rare parameter that only declares one direction.
//
// Rendered through UScrollArea's `virtualize` below rather than a plain UTable: an avatar's full
// parameter list commonly runs into the hundreds, and a non-virtualized table renders every row's
// DOM eagerly regardless of what's actually scrolled into view - painfully slow once names/
// addresses get long. Same reasoning (and the same component) as every other potentially-long list
// in this app - see AvailableParametersList.vue/ExcludedParametersModal.vue.
const rows = computed<ParameterRow[]>(() => {
  const parameters = avatarDetails.value?.parameters ?? []
  const query = search.value.trim().toLowerCase()

  const filtered = parameters
    .filter((param) => showAll.value || !isNoisyAddress(param.name))
    .map((param) => {
      const address = param.output?.address ?? param.input?.address ?? ''
      const received = address in args.value

      return {
        name: param.name,
        address,
        kind: param.output?.type ?? param.input?.type,
        received,
        value: received ? args.value[address]?.[0] : undefined
      }
    })
    .filter(
      (row) => !query || row.name.toLowerCase().includes(query) || row.address.toLowerCase().includes(query)
    )

  const dir = sortDesc.value ? -1 : 1
  return filtered.sort((a, b) => dir * a.name.localeCompare(b.name))
})

const receivedCount = computed(() => rows.value.filter((row) => row.received).length)

// UScrollArea's root uses a plain native scrollbar (overflow-y-auto, no scrollbar-gutter:stable),
// which reserves horizontal space out of its own content box whenever a scrollbar is actually
// shown - narrowing the rows relative to the header row above it, which isn't inside that same
// scrolling element and never loses that space. Measuring and padding the header to match is the
// standard fix for this (used by most virtualized-list libraries).
//
// A ResizeObserver on the scroll area itself (rather than e.g. a window 'resize' listener, or
// re-measuring only when `rows` changes) is what actually covers every case that changes its
// rendered size: the modal's own open/scale-in animation settling after mount, the scrollbar
// appearing/disappearing as filtered rows change, and the window resizing - a 'resize' listener
// alone only catches that last one.
const scrollArea = useTemplateRef('scrollArea')
const scrollbarWidth = ref(0)
let resizeObserver: ResizeObserver | undefined

const measureScrollbarWidth = (el: HTMLElement): void => {
  scrollbarWidth.value = el.offsetWidth - el.clientWidth
}

watch(
  scrollArea,
  (instance) => {
    resizeObserver?.disconnect()
    resizeObserver = undefined

    const el = instance?.$el as HTMLElement | undefined
    if (!el) return

    measureScrollbarWidth(el)
    resizeObserver = new ResizeObserver(() => measureScrollbarWidth(el))
    resizeObserver.observe(el)
  },
  { immediate: true }
)

onUnmounted(() => resizeObserver?.disconnect())

const forcePull = async (missingOnly: boolean): Promise<void> => {
  pulling.value = true

  try {
    const count = await api.forcePullParameters(missingOnly)
    toast.add({
      title: count > 0 ? `Pulled ${count} parameter(s) from VRChat` : 'Nothing to pull',
      description:
        count > 0
          ? undefined
          : missingOnly
            ? 'Every visible parameter has already been received.'
            : undefined,
      icon: 'i-lucide-check',
      color: 'success'
    })
  } catch (err) {
    toast.add({
      title: 'Failed to pull parameters',
      description: String(err),
      icon: 'i-lucide-triangle-alert',
      color: 'error'
    })
  } finally {
    pulling.value = false
  }
}
</script>

<template>
  <UModal
    v-model:open="open"
    :ui="{ content: 'max-w-4xl', body: 'flex flex-col gap-4' }"
    title="Avatar Parameters"
  >
    <template #body>
      <div class="flex items-center justify-between gap-2">
        <span class="text-sm text-muted">
          {{ receivedCount }} / {{ rows.length }} received
        </span>
        <div class="flex shrink-0 gap-2">
          <UTooltip>
            <UButton
              icon="i-lucide-download"
              color="primary"
              variant="subtle"
              :loading="pulling"
              @click="forcePull(true)"
            >
              Force Pull Missing
            </UButton>

            <template #content>
              Fetches current values, straight from VRChat over OSCQuery, for every parameter
              shown below that hasn't been received yet.
            </template>
          </UTooltip>
          <UTooltip>
            <UButton
              icon="i-lucide-refresh-cw"
              color="neutral"
              variant="subtle"
              :loading="pulling"
              @click="forcePull(false)"
            >
              Pull All
            </UButton>

            <template #content>
              Re-fetches every parameter's current value from VRChat, overwriting anything
              already received.
            </template>
          </UTooltip>
        </div>
      </div>

      <div class="flex items-center justify-between gap-2">
        <UInput
          v-model="search"
          icon="i-lucide-search"
          placeholder="Search parameters..."
          class="max-w-64"
        />
        <AddressListToggle v-model="showAll" />
      </div>

      <div
        class="grid grid-cols-[1fr_5rem_8rem] gap-2 border-b border-default px-2 pb-2"
        :style="{ paddingRight: `${scrollbarWidth + 8}px` }"
      >
        <UButton
          color="neutral"
          variant="ghost"
          label="Name"
          :icon="sortIcon"
          class="-mx-2.5 justify-self-start"
          @click="sortDesc = !sortDesc"
        />
        <span class="self-center text-sm font-medium">Type</span>
        <span class="self-center text-sm font-medium">Value</span>
      </div>

      <p
        v-if="!rows.length"
        class="text-sm text-muted"
      >
        No parameters match{{ showAll ? '' : ' - try "Show raw address list" for noisy parameters' }}.
      </p>
      <UScrollArea
        v-else
        ref="scrollArea"
        :items="rows"
        virtualize
        class="h-128"
        :ui="{ item: 'px-2' }"
      >
        <template #default="{ item: row }">
          <div class="grid grid-cols-[1fr_5rem_8rem] items-center gap-2 border-b border-default py-2">
            <div class="min-w-0">
              <p
                class="truncate text-sm font-medium"
                :title="row.name"
              >
                {{ row.name }}
              </p>
              <code
                class="block truncate text-xs text-muted"
                :title="row.address"
              >{{ row.address }}</code>
            </div>
            <span class="text-xs text-muted">{{ row.kind ?? '—' }}</span>
            <div>
              <span
                v-if="!row.received"
                class="text-xs text-muted"
              >
                Not received yet
              </span>
              <UBadge
                v-else-if="typeof row.value === 'boolean'"
                :color="row.value ? 'success' : 'neutral'"
                variant="subtle"
              >
                {{ row.value ? 'True' : 'False' }}
              </UBadge>
              <span
                v-else
                class="text-sm"
              >{{ row.value }}</span>
            </div>
          </div>
        </template>
      </UScrollArea>
    </template>
  </UModal>
</template>

<style scoped></style>
