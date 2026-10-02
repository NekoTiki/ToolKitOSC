<script setup lang="ts">
import type { DropdownMenuItem } from '@nuxt/ui/components/DropdownMenu.vue'
import type { NavigationMenuItem } from '@nuxt/ui/components/NavigationMenu.vue'

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

// Computeds, not plain arrays built once at setup - they need to react to isAdmin resolving
// asynchronously after the initial render (and to the user's own name/avatar changing).
const links = computed<NavigationMenuItem[]>(() => [
  { label: 'Home', icon: 'i-lucide-house', to: '/' },
  ...(loggedIn.value && user.value?.discord ? [{ label: 'Connect app', icon: 'i-lucide-link', to: '/connect' }] : []),
  ...(isAdmin.value ? [{ label: 'Dashboard', icon: 'i-lucide-layout-dashboard', to: '/dashboard' }] : [])
])

// The account menu repeats the page links, so they stay reachable on phones where the header's
// link row is hidden.
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
      to: '/connect'
    },
    ...(isAdmin.value
      ? [{ label: 'Dashboard', icon: 'i-lucide-layout-dashboard', to: '/dashboard' }]
      : [])
  ],
  [
    {
      label: 'Sign out',
      icon: 'i-lucide-log-out',
      color: 'error',
      onSelect: async () => {
        await clear()
        await navigateTo('/')
      }
    }
  ]
])
</script>

<template>
  <UApp>
    <!-- Glass header over the Aurora background (see shared-ui's styles/aurora.css). Links are
    plain buttons, not hover menus, so the site works from a phone or a VR browser. -->
    <UHeader
      to="/"
      :ui="{ root: 'border-default bg-(--ui-bg)/55 backdrop-blur-(--aurora-blur)' }"
    >
      <template #title>
        <span class="flex items-center gap-2.5 font-semibold text-highlighted">
          <span class="size-6.5 rounded-lg bg-linear-to-br from-primary to-secondary" />
          VRC OSC Toolkit
        </span>
      </template>

      <UNavigationMenu
        :items="links"
        variant="pill"
        highlight
      />

      <template #toggle>
        <div />
      </template>

      <template #right>
        <UButton
          v-if="!loggedIn || !user?.discord"
          icon="ic:baseline-discord"
          target="_top"
          href="/auth/discord"
          class="bg-[#5865f2] text-white hover:bg-[#4752c4]"
        >
          Sign in
        </UButton>
        <UDropdownMenu
          v-else
          :items="items"
          :content="{ align: 'end', side: 'bottom', sideOffset: 8 }"
        >
          <UButton
            color="neutral"
            variant="ghost"
            :avatar="{ src: user.discord.avatar, alt: user.discord.name }"
            :label="user.discord.name"
            :ui="{ label: 'max-sm:hidden' }"
          />
        </UDropdownMenu>
      </template>
    </UHeader>
    <UMain>
      <NuxtPage />
    </UMain>
  </UApp>
</template>
