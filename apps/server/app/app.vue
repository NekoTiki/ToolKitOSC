<script setup lang="ts">
import type { DropdownMenuItem } from '@nuxt/ui/components/DropdownMenu.vue'

const { loggedIn, user, clear } = useUserSession()

useHead({ title: 'VRC OSC Toolkit' })

const { refresh } = useFetch('/auth/refresh', { immediate: false })

onMounted(() => refresh())

const items = ref<DropdownMenuItem[][]>([
  [
    {
      label: user.value?.discord?.name,
      avatar: {
        src: user.value?.discord?.avatar
      },
      type: 'label'
    }
  ],
  [
    {
      label: 'Logout',
      icon: 'i-lucide-log-out',
      color: 'error',
      onSelect: () => clear()
    }
  ]
])
</script>

<template>
  <UApp>
    <UHeader to="/">
      <template #title>
        <span
          class="font-bold bg-clip-text text-transparent bg-linear-to-r from-primary-500 to-primary-200"
        >
          VRC OSC Toolkit
        </span>
      </template>

      <template #toggle>
        <div />
      </template>

      <template #right>
        <UButton
          v-if="!loggedIn || !user?.discord"
          icon="ic:baseline-discord"
          target="_top"
          href="/auth/discord"
        >
          Sign in
        </UButton>
        <UDropdownMenu :items="items" :content="{ align: 'end', side: 'bottom', sideOffset: 8 }">
          <UButton
            v-if="loggedIn && user?.discord"
            icon="fa7-solid:user-alt"
            color="neutral"
            variant="outline"
          />
        </UDropdownMenu>
      </template>
    </UHeader>
    <UMain>
      <NuxtPage />
    </UMain>
  </UApp>
</template>
