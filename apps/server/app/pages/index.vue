<script setup lang="ts">
const { loggedIn, user, session, openInPopup } = useUserSession()
</script>

<template>
  <UContainer class="space-x-4 space-y-4">
    <div class="flex">
      <UColorModeSelect />
    </div>
    <UCard>
      <template #header>
        {{ loggedIn }}
      </template>

      <pre v-if="loggedIn && user?.discord">{{ user }}</pre>
      <div v-else class="flex justify-between items-center">
        Not logged in.
        <UButton icon="ic:baseline-discord" @click="() => openInPopup('/auth/discord')">
          Sign in
        </UButton>
      </div>

      <template #footer>
        <pre>{{ session }}</pre>
      </template>
    </UCard>
    <UButton href="/share/12345"> Go to Room "12345" </UButton>
    <UButton href="/share/190926764598362122"> Go to Room "190926764598362122" </UButton>
  </UContainer>
</template>

<style scoped></style>
