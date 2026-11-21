<script setup lang="ts">
// What admins did to this account. Opening the page loads it fresh.
// The parent passes every subpage the same props; the unused ones stay off the DOM.
defineOptions({ inheritAttrs: false })
defineProps<{ discordId: string }>()
</script>

<template>
  <AdminLogList
    :discord-id="discordId"
    :limit="20"
  />
</template>
