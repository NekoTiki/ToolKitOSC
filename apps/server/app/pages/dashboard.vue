<script setup lang="ts">
// Real authorization is server-side (requireAdmin on every /api/admin/* call, plus useAdminApi.ts
// bouncing on a 403) - this middleware is just UX, so an obviously-logged-out visitor doesn't see
// a dashboard shell about to fail every request.
definePageMeta({ middleware: 'admin' })

const route = useRoute()

const navItems = [
  { label: 'Overview', description: 'Usage and AI stats', icon: 'i-lucide-chart-column', to: '/dashboard/stats' },
  { label: 'Users', description: 'Access and credits', icon: 'i-lucide-users', to: '/dashboard/users' },
  { label: 'Generations', description: 'Live AI request log', icon: 'i-lucide-sparkles', to: '/dashboard/generations' }
]

// A user's page belongs to Users in the navigation.
const isActive = (to: string): boolean => route.path.startsWith(to) || (to === '/dashboard/users' && route.path.startsWith('/dashboard/user/'))
</script>

<template>
  <div class="mx-auto grid w-full max-w-7xl gap-5 px-4 py-6 sm:px-6 lg:grid-cols-[14rem_minmax(0,1fr)]">
    <nav
      aria-label="Dashboard"
      class="flex gap-1.5 overflow-x-auto lg:flex-col"
    >
      <p class="px-2.5 pt-1 pb-2 font-mono text-[10.5px] tracking-widest text-muted uppercase max-lg:hidden">
        Admin
      </p>
      <NuxtLink
        v-for="item in navItems"
        :key="item.to"
        :to="item.to"
        class="grid min-h-14.5 shrink-0 grid-cols-[auto_minmax(0,1fr)] items-center gap-x-2.5 rounded-2xl border px-2.5 py-2 transition-colors"
        :class="isActive(item.to) ? 'border-primary/50 bg-primary/14' : 'border-transparent hover:bg-(--aurora-glass)'"
      >
        <span
          class="row-span-2 grid size-8.5 place-items-center rounded-xl bg-(--aurora-well)"
          :class="isActive(item.to) ? 'text-primary' : 'text-muted'"
        >
          <UIcon
            :name="item.icon"
            class="size-4.5"
          />
        </span>
        <b class="truncate text-[14.5px] font-semibold text-highlighted">{{ item.label }}</b>
        <small class="col-start-2 truncate text-xs text-muted max-lg:hidden">{{ item.description }}</small>
      </NuxtLink>
    </nav>

    <div class="min-w-0">
      <NuxtPage />
    </div>
  </div>
</template>
