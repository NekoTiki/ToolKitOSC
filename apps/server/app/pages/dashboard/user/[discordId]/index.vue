<script setup lang="ts">
import type { UserStatsResponse } from '~/types/adminStats'

// Account: suspension and data, AI access, share link.
// The parent passes every subpage the same props; the unused ones stay off the DOM.
defineOptions({ inheritAttrs: false })
defineProps<{ discordId: string; userStats: UserStatsResponse }>()
defineEmits<{ changed: [] }>()
</script>

<template>
  <div class="grid gap-4.5">
    <AdminAccountPanel
      :discord-id="discordId"
      :suspended-until="userStats.user.suspendedUntil"
      :suspend-reason="userStats.user.suspendReason"
      @changed="$emit('changed')"
      @deleted="navigateTo('/dashboard/users')"
    />

    <AdminUserAccessPanel :discord-id="discordId" />

    <AdminShareLinkPanel
      :discord-id="discordId"
      :links="userStats.shareLinks"
      :access="userStats.user.shareAccess"
      @changed="$emit('changed')"
    />
  </div>
</template>
