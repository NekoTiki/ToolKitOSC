<script setup lang="ts">
// Where the desktop app's sign-in lands after Discord (see server/routes/auth/desktop/*):
// confirming hands the app its token over its waiting WebSocket. Shows a real loading state -
// the old page flashed "failed" until the request came back - and says what to do next.
const requestId = useRoute().query.requestId as string
const { user } = useUserSession()

useHead({ title: 'Desktop sign-in' })

const { data, status } = useFetch<{ success: boolean }>('/auth/desktop/confirm', { query: { requestId } })

const state = computed<'loading' | 'success' | 'failed'>(() => {
  if (status.value === 'idle' || status.value === 'pending') return 'loading'

  return data.value?.success ? 'success' : 'failed'
})
</script>

<template>
  <div class="grid min-h-[calc(100dvh-var(--ui-header-height))] place-items-center p-5">
    <div class="grid w-full max-w-md justify-items-center gap-3.5 rounded-panel glass-strong p-7 text-center">
      <template v-if="state === 'loading'">
        <UIcon
          name="i-lucide-loader-circle"
          class="size-13 animate-spin text-primary"
        />
        <h1 class="text-[26px] font-semibold text-highlighted">
          Finishing sign-in…
        </h1>
        <p class="text-muted">
          Sending your account to the desktop app.
        </p>
      </template>

      <template v-else-if="state === 'success'">
        <UIcon
          name="i-lucide-circle-check"
          class="size-13 text-success"
        />
        <h1 class="text-[26px] font-semibold text-highlighted">
          You're signed in to the desktop app
        </h1>
        <p class="text-muted">
          <template v-if="user?.discord">
            Signed in as <b class="font-semibold text-default">{{ user.discord.name }}</b>.
          </template>
          You can close this tab and go back to the app.
        </p>
        <UButton
          block
          size="lg"
          color="neutral"
          variant="subtle"
          icon="i-lucide-link"
          to="/connect"
        >
          See your share link
        </UButton>
      </template>

      <template v-else>
        <UIcon
          name="i-lucide-circle-x"
          class="size-13 text-error"
        />
        <h1 class="text-[26px] font-semibold text-highlighted">
          This sign-in link expired
        </h1>
        <p class="text-muted">
          Sign-in links only work while the app is waiting for them. In the desktop app, open
          <b class="font-semibold text-default">Settings → Account &amp; server</b> and press <b class="font-semibold text-default">Sign in</b> again.
        </p>
        <UButton
          block
          size="lg"
          color="neutral"
          variant="subtle"
          to="/"
        >
          Back to home
        </UButton>
      </template>
    </div>
  </div>
</template>
