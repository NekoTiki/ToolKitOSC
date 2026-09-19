<script setup lang="ts">
import type { DropdownMenuItem } from '@nuxt/ui/components/DropdownMenu.vue'

const { loggedIn, user, clear } = useUserSession()

useHead({ title: 'VRC OSC Toolkit' })

const { refresh } = useFetch('/auth/refresh', { immediate: false })

onMounted(() => refresh())

// Server-decided, not guessed client-side: reuses the same requireAdmin gate every /api/admin/*
// route uses (see server/api/admin/whoami.get.ts), so the Dashboard link only ever shows for an
// account the server actually considers an admin right now, not a flag baked into the session.
const isAdmin = ref(false)

const checkIsAdmin = async (): Promise<void> => {
  if (!loggedIn.value || !user.value?.discord) {
    isAdmin.value = false
    return
  }

  try {
    await $fetch('/api/admin/whoami')
    isAdmin.value = true
  } catch {
    isAdmin.value = false
  }
}

watch(() => loggedIn.value && !!user.value?.discord, checkIsAdmin, { immediate: true })

// A computed, not a plain ref built once at setup - needs to react to isAdmin resolving
// asynchronously after the initial render (and to the user's own name/avatar changing), unlike
// the static array this replaces.
const items = computed<DropdownMenuItem[][]>(() => [
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
      label: 'Connect Desktop App',
      icon: 'i-lucide-link',
      to: '/profile'
    },
    ...(isAdmin.value
      ? [{ label: 'Dashboard', icon: 'i-lucide-layout-dashboard', to: '/dashboard' }]
      : [])
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
