<script setup lang="ts">
import { encodeShareCode } from '#shared/utils/shortLink'

const { loggedIn, user } = useUserSession()

useHead({
  title: 'Connect the Desktop App',
  meta: [
    {
      name: 'description',
      content: "Point the desktop app's server URL at this server, then sign in from inside it."
    }
  ]
})

// This server's own address, not the desktop app's build-time default - a self-hosted server
// needs its own URL here, not the toolkit's public one. ws(s) shares the same host:port as
// http(s) on this server, so a straight protocol swap is enough (mirrors the desktop app's own
// serverHttpUrl computed, which does the reverse conversion).
const requestUrl = useRequestURL()
const serverWsUrl = computed(() => requestUrl.origin.replace(/^http/, 'ws'))

// The share page's room is keyed by the signed-in Discord id (see the server's host.ts
// getRoomId), so there's nothing to share until that's known. Uses the short `/s/<code>` form
// (see server/routes/s/[code].get.ts) - same link the desktop app's own header hands out.
const shareUrl = computed(() =>
  user.value?.discord?.id ? `${requestUrl.origin}/s/${encodeShareCode(user.value.discord.id)}` : null
)

const toast = useToast()

const copy = async (value: string, label: string): Promise<void> => {
  try {
    await navigator.clipboard.writeText(value)
    toast.add({ title: `${label} copied`, icon: 'i-lucide-check', color: 'success' })
  } catch {
    toast.add({ title: `Could not copy ${label.toLowerCase()}`, color: 'error' })
  }
}

const steps = [
  {
    icon: 'lucide:link',
    title: 'Set the server URL',
    description:
      "Open the desktop app, go to Settings → Connection, and paste this server's address into the Server URL field."
  },
  {
    icon: 'ic:baseline-discord',
    title: 'Sign in from the desktop app',
    description:
      'Click "Sign In" in the desktop app\'s header. It opens Discord in your browser for you to approve, then hands the app a token automatically.'
  },
  {
    icon: 'lucide:check-circle-2',
    title: "You're connected",
    description:
      "The desktop app's header now shows the server as connected and your Discord account signed in - build your controls, then hand out the share link below."
  }
]
</script>

<template>
  <UMain>
    <UPageCTA
      v-if="!loggedIn || !user?.discord"
      title="Sign in to connect the desktop app"
      description="This page needs your Discord account signed in here before it can show you anything to connect to."
      :links="[
        {
          label: 'Sign in with Discord',
          icon: 'ic:baseline-discord',
          href: '/auth/discord',
          target: '_top',
          size: 'lg'
        }
      ]"
    />

    <UContainer
      v-else
      class="flex flex-col gap-8 py-8"
    >
      <div class="flex items-center gap-3">
        <UAvatar
          :src="user.discord.avatar"
          size="lg"
        />
        <div>
          <p class="text-sm text-muted">
            Signed in as
          </p>
          <p class="text-lg font-semibold">
            {{ user.discord.name }}
          </p>
        </div>
      </div>

      <div class="flex flex-col gap-4">
        <h2 class="text-lg font-semibold">
          Connect the desktop app to this server
        </h2>
        <UTimeline :items="steps" />
      </div>

      <UCard>
        <template #header>
          <h3 class="font-semibold">
            Server URL
          </h3>
          <p class="text-sm text-muted">
            Paste this into the desktop app's Settings → Connection → Server URL.
          </p>
        </template>

        <div class="flex items-center gap-2">
          <code class="grow truncate rounded-md bg-elevated px-3 py-2 text-sm">{{ serverWsUrl }}</code>
          <UButton
            icon="i-lucide-copy"
            color="neutral"
            variant="subtle"
            aria-label="Copy Server URL"
            @click="copy(serverWsUrl, 'Server URL')"
          />
        </div>
      </UCard>

      <UCard v-if="shareUrl">
        <template #header>
          <h3 class="font-semibold">
            Your share link
          </h3>
          <p class="text-sm text-muted">
            Once you're connected, hand this out so others can use your controls live.
          </p>
        </template>

        <div class="flex items-center gap-2">
          <code class="grow truncate rounded-md bg-elevated px-3 py-2 text-sm">{{ shareUrl }}</code>
          <UButton
            icon="i-lucide-copy"
            color="neutral"
            variant="subtle"
            aria-label="Copy Share Link"
            @click="copy(shareUrl, 'Share link')"
          />
        </div>
      </UCard>
    </UContainer>
  </UMain>
</template>

<style scoped></style>
