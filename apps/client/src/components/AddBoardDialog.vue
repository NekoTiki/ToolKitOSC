<script setup lang="ts">
import ConfirmDialog from '@renderer/components/ui/ConfirmDialog.vue'
import { ref, watch } from 'vue'

// Adds an ESP32 board that mDNS can't see (VPNs, some guest networks) by its address. Pairing
// itself, and its progress, happen on the Settings page once this closes.
const emit = defineEmits<{ add: [address: string] }>()
const open = defineModel<boolean>('open')

const address = ref('')

watch(open, (value) => {
  if (value) address.value = ''
})

const confirm = (): void => {
  const value = address.value.trim()
  if (!value) return

  emit('add', value)
  open.value = false
}
</script>

<template>
  <ConfirmDialog
    v-model:open="open"
    title="Add a board by address"
    confirm-text="Add"
    confirm-icon="i-lucide-plus"
    :danger="false"
    @confirm="confirm"
    @cancel="open = false"
  >
    <p>
      For networks where boards don't show up on their own, like VPNs. The board's serial monitor prints its
      address. After adding, you'll have 30 seconds to press its <strong>BOOT</strong> button.
    </p>
    <template #fields>
      <UFormField label="Address">
        <UInput
          v-model="address"
          placeholder="192.168.1.40 or tkosc-a1b2c3.local"
          class="w-full"
          autocomplete="off"
          autofocus
          :ui="{ base: 'font-mono' }"
          @keydown.enter="confirm"
        />
      </UFormField>
    </template>
  </ConfirmDialog>
</template>
