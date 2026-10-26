<script setup lang="ts">
useHead({ title: 'Log · Dashboard' })
</script>

<template>
  <div class="grid gap-4">
    <div class="grid gap-0.5">
      <h1 class="text-[28px] leading-tight font-semibold text-highlighted">
        Log
      </h1>
      <p class="text-[13px] text-muted">
        What admins changed, newest first
      </p>
    </div>

    <AdminLogList :limit="300" />
  </div>
</template>
