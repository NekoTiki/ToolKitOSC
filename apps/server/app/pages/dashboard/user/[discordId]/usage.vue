<script setup lang="ts">
import type { UserStatsResponse } from '~/types/adminStats'
import { DAY_RANGES } from '~/types/adminStats'

// The parent fetches with this range, so changing it reloads the stats.
// The parent passes every subpage the same props; the unused ones stay off the DOM.
defineOptions({ inheritAttrs: false })
defineProps<{ discordId: string; userStats: UserStatsResponse }>()
const days = defineModel<number>('days', { required: true })
</script>

<template>
  <div class="grid gap-4.5">
    <AdminSegmented
      v-model="days"
      :items="DAY_RANGES"
      label="Date range"
      class="justify-self-end"
    />

    <AdminUsageStats
      :ai="userStats.ai"
      :controls="userStats.controls"
      :days="days"
      daily-title="AI generations per day"
    />

    <RecentGenerationsCard
      :discord-id="discordId"
      :limit="5"
    />
  </div>
</template>
