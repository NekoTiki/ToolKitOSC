<script setup lang="ts">
// The user's share links, and freeing their custom name (squatting, impersonation). Their random
// link keeps working.
const props = defineProps<{ discordId: string; links: { main: string | null; custom: string | null } }>()
const emit = defineEmits<{ changed: [] }>()

const { adminFetch } = useAdminApi()
const toast = useToast()
const origin = useRequestURL().origin

const confirming = ref(false)
const busy = ref(false)

const freeName = async (): Promise<void> => {
  busy.value = true

  try {
    await adminFetch(`/api/admin/users/${props.discordId}/share-name`, { method: 'DELETE' })
    toast.add({ title: 'Name freed', icon: 'i-lucide-check', color: 'success' })
    emit('changed')
  } finally {
    busy.value = false
    confirming.value = false
  }
}
</script>

<template>
  <UCard :ui="{ body: 'grid gap-3' }">
    <h2 class="text-[17px] font-semibold text-highlighted">
      Share link
    </h2>

    <p
      v-if="!links.main && !links.custom"
      class="text-sm text-muted"
    >
      They haven't shared a link yet.
    </p>

    <dl
      v-else
      class="grid grid-cols-[auto_minmax(0,1fr)] items-baseline gap-x-4 gap-y-2 text-sm"
    >
      <template v-if="links.main">
        <dt class="text-muted">
          Link
        </dt>
        <dd class="truncate font-mono">
          <a
            :href="`${origin}/s/${links.main}`"
            target="_blank"
            class="underline decoration-(--ui-border-accented) underline-offset-3"
          >{{ origin }}/s/{{ links.main }}</a>
        </dd>
      </template>
      <template v-if="links.custom">
        <dt class="text-muted">
          Custom name
        </dt>
        <dd class="flex min-w-0 flex-wrap items-center gap-2">
          <span class="truncate font-mono">{{ links.custom }}</span>
          <UButton
            v-if="!confirming"
            size="sm"
            icon="i-lucide-eraser"
            color="error"
            variant="soft"
            @click="confirming = true"
          >
            Free name
          </UButton>
        </dd>
      </template>
    </dl>

    <div
      v-if="confirming"
      class="flex flex-wrap items-center gap-2 rounded-field border border-error/40 bg-error/10 p-3 text-sm"
    >
      <span class="flex-1">Free "{{ links.custom }}"? Links with this name stop working and anyone can take it.</span>
      <UButton
        size="md"
        color="neutral"
        variant="subtle"
        @click="confirming = false"
      >
        Cancel
      </UButton>
      <UButton
        size="md"
        color="error"
        :loading="busy"
        @click="freeName"
      >
        Free name
      </UButton>
    </div>
  </UCard>
</template>
