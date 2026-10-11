<script setup lang="ts">
import { useParameterListener } from '@renderer/composables/useParameterListener'
import { parameterFamily, parameterName } from '@renderer/utils/automations'
import { useNow } from '@vueuse/core'
import { computed, onBeforeUnmount } from 'vue'

// Listen, next to a parameter picker: press it, touch a contact or grab a PhysBone in VRChat, and
// every parameter that changed is listed, newest first, to pick from. See useParameterListener.
const props = defineProps<{
  // Whether a heard parameter can be picked here; one that can't is listed but not pickable.
  canPick?: (address: string) => boolean
}>()
const emit = defineEmits<{ pick: [address: string] }>()

const { listening, endsAt, heard, start, stop } = useParameterListener()
const now = useNow({ interval: 500 })
const secondsLeft = computed(() => Math.max(0, Math.ceil((endsAt.value - now.value.getTime()) / 1000)))
onBeforeUnmount(stop)

const pick = (address: string): void => {
  emit('pick', address)
  stop()
}
</script>

<template>
  <div class="grid gap-2">
    <div class="flex flex-wrap items-center gap-2.5">
      <UButton
        :icon="listening ? 'i-lucide-square' : 'i-lucide-ear'"
        color="neutral"
        variant="subtle"
        @click="listening ? stop() : start()"
      >
        {{ listening ? `Stop listening (${secondsLeft} s)` : 'Listen' }}
      </UButton>
      <span class="min-w-0 flex-1 text-sm text-muted">
        {{
          listening
            ? 'Touch the contact or move the PhysBone in VRChat now.'
            : 'Not sure which parameter a contact uses? Press Listen, then touch it in VRChat.'
        }}
      </span>
    </div>
    <ul
      v-if="heard.length"
      class="grid max-h-96 gap-1.5 overflow-y-auto"
    >
      <li
        v-for="item in heard"
        :key="item.address"
        class="flex items-center gap-2.5 rounded-field border border-default bg-(--aurora-glass) px-3 py-2"
      >
        <span class="min-w-0 flex-1">
          <b class="block truncate text-sm font-medium text-highlighted">{{ parameterName(item.address) }}</b>
          <small class="text-xs text-muted">
            {{ parameterFamily(item.address) ?? 'Changed' }} · {{ item.count }}× · now {{ item.value }}{{ item.control ? ` · your control ${item.control}` : '' }}
          </small>
        </span>
        <UButton
          size="sm"
          :disabled="props.canPick ? !props.canPick(item.address) : false"
          @click="pick(item.address)"
        >
          Use
        </UButton>
      </li>
    </ul>
    <p
      v-else-if="listening"
      class="text-sm text-muted"
    >
      Nothing has changed yet.
    </p>
  </div>
</template>
