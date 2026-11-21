<script setup lang="ts">
import type { UserStatsResponse } from '~/types/adminStats'

// The parent passes every subpage the same props; the unused ones stay off the DOM.
defineOptions({ inheritAttrs: false })
defineProps<{ userStats: UserStatsResponse }>()
</script>

<template>
  <AdminUserAddonsCard :addons="userStats.addons" />
</template>
