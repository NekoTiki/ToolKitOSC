<script setup lang="ts">
import logoUrl from '@renderer/assets/logo.svg'
import FormSection from '@renderer/components/ui/FormSection.vue'
import { useAppUpdater } from '@renderer/composables/useAppUpdater'
import { getVersion } from '@tauri-apps/api/app'
import { onMounted, ref } from 'vue'

// Update results (up to date, update available with an Update & Restart action, failures) are
// shown as toasts by useAppUpdater itself.
const { checking, checkForUpdates } = useAppUpdater()
const version = ref<string>()

onMounted(() => void getVersion().then((value) => (version.value = value)))
</script>

<template>
  <FormSection>
    <div class="flex flex-wrap items-center gap-3.5">
      <img
        :src="logoUrl"
        alt=""
        class="size-12 rounded-xl"
      >
      <div class="grid min-w-0 flex-1">
        <b class="text-[15px] font-semibold text-highlighted">VRC OSC Toolkit</b>
        <small class="text-[13px] text-muted">{{ version ? `Version ${version}` : 'Checking version…' }}</small>
      </div>
      <UButton
        icon="i-lucide-refresh-cw"
        color="neutral"
        variant="subtle"
        :loading="checking"
        @click="checkForUpdates()"
      >
        Check for updates
      </UButton>
    </div>
  </FormSection>
</template>
