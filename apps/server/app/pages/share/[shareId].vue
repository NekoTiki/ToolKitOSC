<script setup lang="ts">
import type { ControlType } from '@toolkitosc/shared-ui'
import { Control, controlTileSpan, randomGuestName } from '@toolkitosc/shared-ui'

import { useClientTheme } from '~/composables/useClientTheme'
import type { ShareInfo } from '~~/server/api/share/[shareId].get'

const shareId = useRoute().params.shareId as string

// Awaited (not fire-and-forget) so `shareInfo` is already populated when the SEO meta below reads
// it - this is what Discord's link-preview crawler (which never runs JS) actually sees, not the
// generic fallback text a client-side-only fetch would leave in the initial HTML.
const { data: shareInfo } = await useFetch<ShareInfo>(`/api/share/${shareId}`)

const requestUrl = useRequestURL()
const pageUrl = `${requestUrl.origin}/share/${shareId}`

// Drawn per room by server/routes/og/share/[shareId].ts. Versioned by the state it shows, so
// Discord (which caches embed images by URL) fetches a fresh one once the host comes online or
// shares more controls, instead of reusing an "Offline" card from an earlier paste.
const ogImageUrl = computed(() => {
  const info = shareInfo.value
  const version = info?.online ? `on-${info.controlCount}` : 'off'

  return `${requestUrl.origin}/og/share/${shareId}?v=${version}`
})

const title = computed(() =>
  shareInfo.value?.hostName ? `${shareInfo.value.hostName}'s Controls` : 'Shared Controls'
)

const description = computed(() => {
  const info = shareInfo.value
  const who = info?.hostName ? `${info.hostName} is` : 'This host is'

  if (!info?.online) return `${who} currently offline.`
  if (info.controlCount === 0) return `${who} online, but hasn't shared any controls yet.`

  return `${who} sharing ${info.controlCount} live ${info.controlCount === 1 ? 'control' : 'controls'} - open the link to view and use them.`
})

useSeoMeta({
  title,
  description,
  ogTitle: title,
  ogDescription: description,
  ogImage: ogImageUrl,
  ogImageWidth: 1200,
  ogImageHeight: 630,
  ogImageType: 'image/png',
  ogUrl: pageUrl,
  ogType: 'website',
  twitterCard: 'summary_large_image',
  twitterTitle: title,
  twitterDescription: description,
  twitterImage: ogImageUrl
})

// The viewer side of the Cockpit layout (see the desktop client's Controls page): the host's
// groups in a sidebar - or a row of chips on a phone - and the selected group's tiles filling the
// page, in the host's theme. Sign-in, banned, offline and disconnected are full-page states
// rather than modals.
const { controlGroups, authRequired, banned, viewers, you, lastAction, status, hostStatus, open, close, sendMessage } =
  useWebsocketClient(shareId)
const { removeTheme } = useClientTheme()

const connectedOnce = ref(false)
const wsOffline = computed(() => status.value === 'CLOSED')
const hostOffline = computed(() => hostStatus.value !== 'online')

watch(status, (newStatus) => {
  if (newStatus === 'OPEN') connectedOnce.value = true
})

onMounted(open)
onBeforeUnmount(close)
onBeforeUnmount(removeTheme)

const hostName = computed(() => shareInfo.value?.hostName ?? 'This host')

// Each viewer picks their own tile size (their screen, not the host's), remembered per device. A
// cookie, not localStorage: the server reads it too, so SSR renders the saved size - no hydration
// mismatch on the S/M/L buttons and no flash of the wrong tile size.
const density = useCookie<'s' | 'm' | 'l'>('share_tileDensity', { default: () => 'm', maxAge: 60 * 60 * 24 * 365, sameSite: 'lax' })

// The installed web app starts at /launch (see public/manifest.webmanifest), which reopens the last
// share this device viewed - read server-side by server/routes/launch.get.ts, hence a cookie. Set on
// mount rather than during SSR so link-preview crawlers fetching the page never get one. Stored raw
// (useCookie JSON-encodes by default, which would wrap the id in quotes) for that server route.
const lastShare = useCookie<string>('share_last', {
  maxAge: 60 * 60 * 24 * 365,
  sameSite: 'lax',
  encode: (value) => value ?? '',
  decode: (value: string) => value
})

onMounted(() => {
  lastShare.value = shareId
})

const query = ref('')
const search = computed(() => query.value.trim().toLowerCase())
const selectedId = ref<string | null>(null)

const selectedGroup = computed(() => controlGroups.value.find((group) => group.id === selectedId.value) ?? controlGroups.value[0] ?? null)

const shown = computed<{ control: ControlType; groupId: string }[]>(() => {
  if (search.value) {
    return controlGroups.value.flatMap((group) =>
      group.controls.filter((control) => control.name.toLowerCase().includes(search.value)).map((control) => ({ control, groupId: group.id }))
    )
  }

  return selectedGroup.value?.controls.map((control) => ({ control, groupId: selectedGroup.value!.id })) ?? []
})

const selectGroup = (id: string): void => {
  query.value = ''
  selectedId.value = id
}

const totalControls = computed(() => controlGroups.value.reduce((n, group) => n + group.controls.length, 0))

const sendCommand = (groupId: string, control: ControlType, command: object): void => {
  sendMessage('command', { groupId, controlId: control.id, controlName: control.name, ...command })
}

// Username sign-in: the display name goes to the host over WS and into the session. Pre-filled
// with a random name (set on mount, not during SSR, so it can't cause a hydration mismatch) - keep
// it, reroll it, or type your own.
const username = ref('')

const rerollUsername = (): void => {
  username.value = randomGuestName()
}

onMounted(rerollUsername)

const submitUsername = (): void => {
  const name = username.value.trim()

  if (!name) return

  sendMessage('update-username', { displayName: name })
  void $fetch('/auth/set-username', { method: 'POST', body: { username: name } })
  authRequired.value = null
}

// Which full-page state replaces the tiles, if any. Offline with controls still shows them, dimmed
// under a banner, so viewers see what will come back.
const pageState = computed<'banned' | 'signin' | 'connecting' | 'disconnected' | 'offline-empty' | 'empty' | null>(() => {
  if (banned.value) return 'banned'
  if (authRequired.value) return 'signin'
  if (!connectedOnce.value) return 'connecting'
  if (wsOffline.value) return 'disconnected'
  if (!controlGroups.value.length) return hostOffline.value ? 'offline-empty' : 'empty'

  return null
})
</script>

<template>
  <div class="flex h-[calc(100dvh-var(--ui-header-height))] flex-col">
    <div class="flex min-h-15.5 shrink-0 items-center gap-2.5 border-b border-default px-4">
      <div class="grid min-w-0 flex-1">
        <span class="truncate text-[22px] leading-tight font-semibold text-highlighted">{{ hostName }}'s controls</span>
        <span class="truncate font-mono text-[11px] text-muted">{{ totalControls }} controls shared</span>
      </div>
      <span class="flex h-10.5 shrink-0 items-center gap-2 rounded-2xl glass px-3 text-sm font-medium">
        <span
          class="size-2 rounded-full"
          :class="hostOffline ? 'bg-error/70' : 'bg-success shadow-[0_0_6px_var(--ui-success)]'"
        />
        <span class="max-sm:hidden">{{ hostOffline ? 'Host offline' : 'Host online' }}</span>
      </span>
      <!-- Phones have no sidebar, so the list opens from here. -->
      <UPopover
        v-if="viewers"
        :content="{ align: 'end' }"
      >
        <button
          type="button"
          class="flex h-10.5 shrink-0 cursor-pointer items-center gap-2 rounded-2xl glass px-3 text-sm font-medium"
          :aria-label="`${viewers.length} here now`"
        >
          <UIcon
            name="i-lucide-users"
            class="size-4"
          />
          {{ viewers.length }}
        </button>
        <template #content>
          <ShareViewers
            :viewers="viewers"
            :you="you"
            class="w-72 p-2"
          />
        </template>
      </UPopover>
      <div
        class="flex shrink-0 gap-0.5 rounded-[calc(var(--ui-radius)*4)] glass p-1 max-sm:hidden"
        role="group"
        aria-label="Tile size"
      >
        <button
          v-for="size in (['s', 'm', 'l'] as const)"
          :key="size"
          type="button"
          class="h-10 min-w-11 cursor-pointer rounded-2xl px-3 text-sm font-medium uppercase transition-colors"
          :class="density === size ? 'bg-accented text-highlighted' : 'text-muted hover:text-default'"
          :aria-pressed="density === size"
          @click="density = size"
        >
          {{ size }}
        </button>
      </div>
    </div>

    <div class="relative grid min-h-0 flex-1 lg:grid-cols-[16.5rem_minmax(0,1fr)]">
      <aside class="flex min-h-0 flex-col gap-2.5 border-r border-default p-3.5 max-lg:hidden">
        <UInput
          v-model="query"
          icon="i-lucide-search"
          placeholder="Search controls"
          :ui="{ base: 'rounded-2xl' }"
        />
        <nav
          aria-label="Groups"
          class="-mr-1.5 flex min-h-0 flex-1 flex-col gap-1.5 overflow-y-auto pr-1.5"
        >
          <button
            v-for="group in controlGroups"
            :key="group.id"
            type="button"
            class="grid min-h-14.5 cursor-pointer grid-cols-[auto_minmax(0,1fr)] items-center gap-x-2.5 rounded-2xl border px-2.5 py-2 text-left transition-colors"
            :class="!search && group.id === selectedGroup?.id ? 'border-primary/50 bg-primary/14' : 'border-transparent hover:bg-(--aurora-glass)'"
            :title="group.name"
            @click="selectGroup(group.id)"
          >
            <span
              class="row-span-2 grid size-8.5 place-items-center rounded-xl bg-(--aurora-well)"
              :class="!search && group.id === selectedGroup?.id ? 'text-primary' : 'text-muted'"
            >
              <UIcon
                name="i-lucide-folder"
                class="size-4.5"
              />
            </span>
            <b class="truncate text-[14.5px] font-semibold text-highlighted">{{ group.name }}</b>
            <small class="col-start-2 truncate text-xs text-muted">{{ group.controls.length }} controls</small>
          </button>
        </nav>
        <ShareViewers
          v-if="viewers?.length"
          :viewers="viewers"
          :you="you"
          class="shrink-0 border-t border-default pt-1"
        />
      </aside>

      <section class="relative flex min-h-0 min-w-0 flex-col">
        <!-- Phones get a scrolling row of group chips instead of the sidebar. -->
        <nav
          v-if="controlGroups.length"
          aria-label="Groups"
          class="flex shrink-0 gap-2 overflow-x-auto px-4 pt-3 lg:hidden"
        >
          <button
            v-for="group in controlGroups"
            :key="group.id"
            type="button"
            class="flex h-11.5 shrink-0 cursor-pointer items-center gap-2 rounded-full border px-4 text-sm font-medium whitespace-nowrap transition-colors"
            :class="group.id === selectedGroup?.id ? 'border-primary/55 bg-primary/18' : 'border-default bg-(--aurora-glass)'"
            @click="selectGroup(group.id)"
          >
            {{ group.name.length > 24 ? `${group.name.slice(0, 23)}…` : group.name }}
            <small class="font-mono text-[11px] text-muted">{{ group.controls.length }}</small>
          </button>
        </nav>

        <div
          v-if="!pageState && hostOffline"
          class="mx-4 mt-3 flex items-center gap-3 rounded-field border border-warning/40 bg-warning/10 px-3.5 py-3 text-sm lg:mx-5.5"
        >
          <UIcon
            name="i-lucide-wifi-off"
            class="size-5 shrink-0 text-warning"
          />
          <span>{{ hostName }} is offline. Controls are paused and come back by themselves.</span>
        </div>

        <div
          v-if="!pageState"
          class="flex min-w-0 shrink-0 items-center gap-3 px-4 pt-3.5 pb-3 max-lg:hidden lg:px-5.5"
        >
          <h2
            class="truncate text-[28px] leading-tight font-semibold text-highlighted"
            :title="search ? 'Search' : selectedGroup?.name"
          >
            {{ search ? 'Search' : selectedGroup?.name }}
          </h2>
          <span class="shrink-0 text-[13px] text-muted">{{ shown.length }} controls</span>
        </div>

        <div
          v-if="!pageState"
          class="min-h-0 flex-1 overflow-auto px-4 pb-5.5 max-lg:pt-3 lg:px-5.5"
          :class="{ 'max-lg:pb-20': lastAction }"
        >
          <div
            v-if="shown.length"
            class="tile-grid max-sm:tile-grid-fluid"
            :data-density="density"
          >
            <div
              v-for="{ control, groupId } in shown"
              :key="control.id"
              :data-span="controlTileSpan(control)"
            >
              <Control
                :control="control"
                :locked="control.locked"
                :unavailable="control.unavailable"
                :limits="control.limits"
                :offline="hostOffline"
                @command="sendCommand(groupId, control, $event)"
              />
            </div>
          </div>
          <p
            v-else
            class="py-12 text-center text-muted"
          >
            {{ search ? `Nothing matches "${query.trim()}".` : 'No controls in this group.' }}
          </p>
        </div>

        <ShareStateCard
          v-else-if="pageState === 'connecting'"
          icon="i-lucide-loader-circle"
          title="Connecting"
          :description="`Joining ${hostName}'s controls…`"
          spin
        />
        <ShareStateCard
          v-else-if="pageState === 'disconnected'"
          icon="i-lucide-cloud-off"
          title="You're disconnected"
          description="Check your internet connection, then reconnect."
        >
          <UButton
            size="xl"
            block
            icon="i-lucide-refresh-cw"
            @click="open"
          >
            Reconnect
          </UButton>
        </ShareStateCard>
        <ShareStateCard
          v-else-if="pageState === 'signin'"
          icon="i-lucide-user-round"
          title="Sign in to join"
          :description="`${hostName} asks viewers to sign in, so everyone can see who pressed what.`"
        >
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
          <template v-if="authRequired === 'username'">
            <span class="font-mono text-[11px] tracking-widest text-muted uppercase">or pick a display name</span>
            <form
              class="flex gap-2"
              @submit.prevent="submitUsername"
            >
              <UInput
                v-model="username"
                size="xl"
                placeholder="Display name"
                aria-label="Display name"
                class="min-w-0 flex-1"
              />
              <UButton
                type="button"
                size="xl"
                color="neutral"
                variant="subtle"
                icon="i-lucide-dices"
                aria-label="Random name"
                title="Random name"
                @click="rerollUsername"
              />
              <UButton
                type="submit"
                size="xl"
                :disabled="!username.trim()"
              >
                Join
              </UButton>
            </form>
          </template>
        </ShareStateCard>
        <ShareStateCard
          v-else-if="pageState === 'banned'"
          icon="i-lucide-ban"
          tone="error"
          title="You can't use these controls"
          :description="`${hostName} blocked your ${banned?.scope === 'discord' ? 'Discord account' : 'IP address'} from this share page.`"
        >
          <p
            v-if="banned?.reason"
            class="rounded-field well px-3.5 py-2.5 text-left text-sm"
          >
            <b class="font-semibold">Reason:</b> {{ banned.reason }}
          </p>
        </ShareStateCard>
        <ShareStateCard
          v-else-if="pageState === 'offline-empty'"
          icon="i-lucide-wifi-off"
          :title="`${hostName} is offline`"
          description="Their controls show up here by themselves when they open the desktop app."
        />
        <ShareStateCard
          v-else
          icon="i-lucide-inbox"
          title="Nothing shared yet"
          :description="`${hostName} is online but hasn't shared any controls. This page updates by itself.`"
        />

        <!-- Under the tiles on a desktop; a floating pill over them on a phone. -->
        <ShareLastAction
          v-if="!pageState && lastAction"
          :action="lastAction"
          :control-groups="controlGroups"
          :you="you"
          class="mx-5.5 mb-3.5 shrink-0 rounded-2xl max-lg:absolute max-lg:inset-x-3.5 max-lg:bottom-3.5 max-lg:z-10 max-lg:m-0 max-lg:h-12 max-lg:rounded-full max-lg:bg-(--ui-bg) max-lg:shadow-[0_10px_30px_-10px_rgba(0,0,0,.6)]"
        />
      </section>
    </div>
  </div>
</template>
