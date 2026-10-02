<script setup lang="ts">
import { encodeShareCode } from '#shared/utils/shortLink'
import type { ShareInfo } from '~~/server/api/share/[shareId].get'

const { loggedIn, user, clear } = useUserSession()

useHead({
  title: 'Connect the desktop app',
  meta: [{ name: 'description', content: "Point the desktop app's server URL at this server, then sign in from inside it." }]
})

// This server's own address, not the desktop app's build-time default - a self-hosted server
// needs its own URL here. ws(s) shares host:port with http(s), so a protocol swap is enough.
const requestUrl = useRequestURL()
const serverWsUrl = computed(() => requestUrl.origin.replace(/^http/, 'ws'))

// The share room is keyed by the Discord id (see the server's host.ts getRoomId); the link uses
// the short, reversible `/s/<code>` form of it.
const discordId = computed(() => user.value?.discord?.id ?? null)
const shareUrl = computed(() => (discordId.value ? `${requestUrl.origin}/s/${encodeShareCode(discordId.value)}` : null))

// Whether this user's desktop app is connected right now - the same room lookup the share page's
// link preview uses, so no dedicated endpoint is needed.
const { data: hostInfo } = useFetch<ShareInfo>(() => `/api/share/${discordId.value}`, { immediate: !!discordId.value, watch: [discordId], server: false })

const toast = useToast()

const copy = async (value: string, label: string): Promise<void> => {
  try {
    await navigator.clipboard.writeText(value)
    toast.add({ title: `${label} copied`, icon: 'i-lucide-check', color: 'success' })
  } catch {
    toast.add({ title: `Could not copy the ${label.toLowerCase()}`, color: 'error' })
  }
}

const signOut = async (): Promise<void> => {
  await clear()
  await navigateTo('/')
}

const connected = computed(() => !!hostInfo.value?.online)

const steps = computed(() => [
  { title: 'Set the server URL', description: "In the desktop app, open Settings → Account & server and paste this server's address below.", done: connected.value },
  { title: 'Sign in from the desktop app', description: 'Press Sign in in the app. Your browser opens this site to finish, then hands the app a token automatically.', done: connected.value },
  { title: "You're connected", description: 'Build your controls in the app, then share the link below.', done: connected.value }
])
</script>

<template>
  <div class="mx-auto grid w-full max-w-3xl gap-4 px-4 py-8 sm:px-7">
    <div
      v-if="!loggedIn || !user?.discord"
      class="mx-auto mt-10 grid w-full max-w-md justify-items-center gap-3.5 rounded-panel glass-strong p-7 text-center"
    >
      <UIcon
        name="i-lucide-link"
        class="size-12 text-muted"
      />
      <h1 class="text-[26px] font-semibold text-highlighted">
        Sign in to connect the desktop app
      </h1>
      <p class="text-muted">
        Your share link and the app's sign-in both use your Discord account.
      </p>
      <UButton
        size="xl"
        block
        icon="ic:baseline-discord"
        target="_top"
        href="/auth/discord"
        class="bg-[#5865f2] text-white hover:bg-[#4752c4]"
      >
        Sign in with Discord
      </UButton>
    </div>

    <template v-else>
      <section class="flex flex-wrap items-center gap-4 rounded-panel glass p-4.5">
        <UAvatar
          :src="user.discord.avatar"
          :alt="user.discord.name"
          size="3xl"
        />
        <div class="grid min-w-0 flex-1">
          <small class="text-muted">Signed in as</small>
          <b class="truncate text-xl font-semibold text-highlighted">{{ user.discord.name }}</b>
        </div>
        <UButton
          icon="i-lucide-log-out"
          color="error"
          variant="soft"
          @click="signOut"
        >
          Sign out
        </UButton>
      </section>

      <div
        class="flex items-center gap-3 rounded-field border px-4 py-3 text-sm"
        :class="connected ? 'border-success/40 bg-success/10' : 'border-default bg-(--aurora-glass)'"
      >
        <UIcon
          :name="connected ? 'i-lucide-circle-check' : 'i-lucide-monitor'"
          class="size-5 shrink-0"
          :class="connected ? 'text-success' : 'text-muted'"
        />
        <div>
          {{ connected ? 'Your desktop app is connected' : "Your desktop app isn't connected right now" }}
          <small class="block text-muted">{{ connected ? `${hostInfo?.controlCount ?? 0} controls shared` : 'Follow the steps below, or open the app if it is already set up.' }}</small>
        </div>
      </div>

      <section class="grid gap-3.5 rounded-panel glass p-4.5">
        <h2 class="text-[17px] font-semibold text-highlighted">
          Connect the desktop app to this server
        </h2>
        <ol class="grid">
          <li
            v-for="(step, index) in steps"
            :key="step.title"
            class="relative grid grid-cols-[2.5rem_minmax(0,1fr)] gap-3.5 pb-4.5 last:pb-0"
          >
            <span
              v-if="index < steps.length - 1"
              class="absolute top-11 bottom-1 left-[1.2rem] w-0.5 bg-(--ui-border)"
              aria-hidden="true"
            />
            <span
              class="grid size-10 place-items-center rounded-full border-2 font-semibold"
              :class="step.done ? 'border-success bg-success text-(--ui-bg)' : 'border-(--ui-border-accented) bg-(--aurora-well)'"
            >
              <UIcon
                v-if="step.done"
                name="i-lucide-check"
                class="size-5"
              />
              <template v-else>{{ index + 1 }}</template>
            </span>
            <div class="pt-2">
              <b class="block font-semibold text-highlighted">{{ step.title }}</b>
              <p class="mt-1 text-[13.5px] leading-relaxed text-muted">
                {{ step.description }}
              </p>
            </div>
          </li>
        </ol>
      </section>

      <section class="grid gap-3 rounded-panel glass p-4.5">
        <div>
          <h2 class="text-[17px] font-semibold text-highlighted">
            Server URL
          </h2>
          <p class="text-[13px] text-muted">
            Paste this in the desktop app's server settings.
          </p>
        </div>
        <div class="flex min-h-13.5 items-center gap-2 rounded-field well py-1.5 pr-1.5 pl-3.5 font-mono text-[13px]">
          <span class="min-w-0 flex-1 truncate">{{ serverWsUrl }}</span>
          <UButton
            icon="i-lucide-copy"
            color="neutral"
            variant="subtle"
            @click="copy(serverWsUrl, 'Server URL')"
          >
            Copy
          </UButton>
        </div>
      </section>

      <section
        v-if="shareUrl"
        class="grid gap-3 rounded-panel glass p-4.5"
      >
        <div>
          <h2 class="text-[17px] font-semibold text-highlighted">
            Your share link
          </h2>
          <p class="text-[13px] text-muted">
            Anyone with this link sees the groups you haven't hidden.
          </p>
        </div>
        <div class="flex min-h-13.5 items-center gap-2 rounded-field well py-1.5 pr-1.5 pl-3.5 font-mono text-[13px]">
          <span class="min-w-0 flex-1 truncate">{{ shareUrl }}</span>
          <UButton
            icon="i-lucide-copy"
            color="neutral"
            variant="subtle"
            @click="copy(shareUrl, 'Share link')"
          >
            Copy
          </UButton>
          <UButton
            icon="i-lucide-external-link"
            :to="shareUrl"
          >
            Open
          </UButton>
        </div>
      </section>
    </template>
  </div>
</template>
