<script setup lang="ts">
import EmptyState from '@renderer/components/ui/EmptyState.vue'
import { useAddons } from '@renderer/composables/useAddons'
import { useAddonSwitch } from '@renderer/composables/useAddonSwitch'
import { computed, ref } from 'vue'

// Shown in place of a page or settings section whose add-on is off, e.g. a link to /patterns while
// Intiface toys is off.
const props = defineProps<{ addonId: string }>()

const { getAddon } = useAddons()
const { setAddon } = useAddonSwitch()

const addon = computed(() => getAddon(props.addonId))
const turningOn = ref(false)

const turnOn = async (): Promise<void> => {
  if (!addon.value) return
  turningOn.value = true
  try {
    await setAddon(addon.value, true)
  } finally {
    turningOn.value = false
  }
}
</script>

<template>
  <EmptyState
    icon="i-lucide-puzzle"
    :title="`${addon?.name ?? 'This add-on'} is off`"
    :description="addon ? `${addon.pitch}. Turn it on to use this page.` : undefined"
  >
    <template #actions>
      <UButton
        icon="i-lucide-power"
        size="xl"
        class="rounded-2xl"
        :loading="turningOn"
        @click="turnOn"
      >
        Turn on {{ addon?.name ?? '' }}
      </UButton>
      <UButton
        to="/settings/addons"
        color="neutral"
        variant="subtle"
        size="xl"
        class="rounded-2xl"
      >
        All add-ons
      </UButton>
    </template>
  </EmptyState>
</template>
