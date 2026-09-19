<script setup lang="ts">
// Real authorization is server-side (requireAdmin on every /api/admin/* call, plus useAdminApi.ts
// bouncing on a 403) - this middleware is just UX, so an obviously-logged-out visitor doesn't see
// a dashboard shell about to fail every request.
definePageMeta({ middleware: 'admin' })

const navItems = [
  { label: 'Users', icon: 'i-lucide-users', to: '/dashboard/users' },
  { label: 'Stats', icon: 'i-lucide-bar-chart-3', to: '/dashboard/stats' }
]
</script>

<template>
  <div class="mx-auto flex w-full max-w-6xl flex-col gap-6 p-4 sm:flex-row">
    <UNavigationMenu
      :items="navItems"
      orientation="vertical"
      class="w-full shrink-0 sm:w-48"
    />
    <div class="min-w-0 flex-1">
      <NuxtPage />
    </div>
  </div>
</template>
