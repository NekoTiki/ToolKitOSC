<script setup lang="ts">
const requestId = useRoute().query.requestId as string

useHead({
  title: 'Desktop Authentication'
})

const { data } = useFetch('/auth/desktop/confirm', {
  query: { requestId }
})
</script>

<template>
  <UMain class="gap-4 flex flex-col justify-center py-4">
    <UEmpty
      v-if="data?.success"
      icon="i-lucide-check-circle-2"
      size="xl"
      title="Logged In Successfully"
      description="Desktop authentication completed."
      class="h-min self-center"
    />
    <UEmpty
      v-else
      icon="i-lucide-x-circle"
      size="xl"
      title="Authentication Failed"
      description="The desktop authentication request was denied or has expired."
      class="h-min self-center"
    />
  </UMain>
</template>

<style scoped></style>
