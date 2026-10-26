<script setup lang="ts">
import type { NuxtError } from '#app'

// Replaces app.vue (and its header) for any error, so it carries its own small brand bar.
const props = defineProps<{ error: NuxtError & { url?: string } }>()

const route = useRoute()
const notFound = computed(() => props.error.statusCode === 404)
// A reset or made-up share link (see server/db/shareLinks.ts): say so, instead of a bare 404.
// Server errors carry the requested path; errors thrown in the browser are on the current route.
const deadShareLink = computed(() => notFound.value && /^\/(share|s)\//.test(props.error.url ?? route.path))

const title = computed(() => {
  if (deadShareLink.value) return "This share link doesn't work"
  if (notFound.value) return 'Page not found'

  return 'Something went wrong'
})

const description = computed(() => {
  if (deadShareLink.value) return 'The host may have reset their link, or it was typed wrong. Ask them for their current link.'
  if (notFound.value) return "There's nothing at this address."

  return 'The server ran into a problem. Try again in a moment.'
})

useHead({ title })
</script>

<template>
  <UApp>
    <div class="grid min-h-dvh grid-rows-[auto_1fr]">
      <header class="flex h-(--ui-header-height) items-center border-b border-default bg-(--ui-bg)/55 px-4 backdrop-blur-(--aurora-blur) sm:px-6">
        <NuxtLink
          to="/"
          class="flex items-center gap-2.5 font-semibold text-highlighted"
        >
          <span class="size-6.5 rounded-lg bg-linear-to-br from-primary to-secondary" />
          ToolKitOSC
        </NuxtLink>
      </header>

      <main class="grid place-items-center px-4 py-10">
        <div class="grid w-full max-w-md justify-items-center gap-3.5 rounded-panel glass-strong p-7 text-center">
          <UIcon
            :name="deadShareLink ? 'i-lucide-link-2-off' : notFound ? 'i-lucide-map-pin-off' : 'i-lucide-triangle-alert'"
            class="size-12 text-muted"
          />
          <small class="font-mono text-muted">{{ error.statusCode }}</small>
          <h1 class="text-[26px] font-semibold text-highlighted">
            {{ title }}
          </h1>
          <p class="text-muted">
            {{ description }}
          </p>
          <UButton
            size="xl"
            block
            icon="i-lucide-house"
            @click="clearError({ redirect: '/' })"
          >
            Go to the home page
          </UButton>
        </div>
      </main>
    </div>
  </UApp>
</template>
