<script setup lang="ts">
defineProps<{
  method: 'discord' | 'username' | null
}>()

const open = defineModel<boolean>('open')
defineEmits(['update:username'])

const username = ref('')
</script>

<template>
  <UModal v-model:open="open" title="Authentication" class="w-[400px]">
    <template #body>
      <div class="flex flex-col gap-4">
        <div v-if="method === 'username'" class="flex gap-2">
          <UInput v-model="username" class="flex-grow" placeholder="Username" />
          <UButton icon="lucide:check" @click="$emit('update:username', username)">
            Access
          </UButton>
        </div>
        <USeparator v-if="method === 'username'" label="or" class="h-full" />
        <div class="flex justify-center">
          <UButton icon="ic:baseline-discord" target="_top" href="/auth/discord"> Sign in </UButton>
        </div>
      </div>
    </template>
  </UModal>
</template>

<style scoped></style>
