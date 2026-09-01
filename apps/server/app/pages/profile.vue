<script setup lang="ts">
const { loggedIn, user, session } = useUserSession()
</script>

<template>
  <UContainer>
    <div>{{ loggedIn }}</div>
    <div>{{ user }}</div>
    <div>{{ session }}</div>
  </UContainer>
</template>

<style scoped></style>
