<script setup lang="ts">
import { useAuth } from '@renderer/composables/useAuth'
import { useAvatarDetails } from '@renderer/composables/useAvatarDetails'
import { useControls } from '@renderer/composables/useControls'
import { useLockedControls } from '@renderer/composables/useLockedControls'
import { useWebsocketAuth } from '@renderer/composables/useWebsocketAuth'
import { serverHttpUrl } from '@renderer/composables/useWebsocketSettings'
import { api } from '@renderer/lib/tauri-bridge'
import { encodeShareCode } from '@toolkitosc/shared-ui'
import { computed } from 'vue'
import { useRouter } from 'vue-router'

// Global bar under the title bar: which avatar is loaded, the active lock profile, connection
// status (StatusBar.vue), the share link and your account. Every chip opens its page, as in the
// redesign. Page-specific actions live in each page's header.
const { avatarDetails } = useAvatarDetails()
const { controls } = useControls()
const { user, loggedIn } = useAuth()
const { authUrl } = useWebsocketAuth()
const { lockedControlGroups, currentLockedControlsGroup } = useLockedControls()
const toast = useToast()
const router = useRouter()

const controlCount = computed(() => controls.value.reduce((n, group) => n + group.controls.length, 0))

const activeProfile = computed(
  () => lockedControlGroups.value.find((group) => group.id === currentLockedControlsGroup.value) ?? null
)

// The share page's room is keyed by the host's Discord id (see the server's host.ts `getRoomId`),
// so there's nothing to share until that's known. `/s/<code>` is a reversible re-encoding of that
// id (see server/routes/s/[code].get.ts), so it needs no round trip to produce.
const shareUrl = computed<string | null>(() =>
  user.value?.discord?.id ? `${serverHttpUrl.value}/s/${encodeShareCode(user.value.discord.id)}` : null
)

const copyShareLink = async (): Promise<void> => {
  if (!shareUrl.value) return

  try {
    await navigator.clipboard.writeText(shareUrl.value)
    toast.add({ title: 'Share link copied', icon: 'i-lucide-check', color: 'success' })
  } catch {
    toast.add({ title: 'Could not copy share link', color: 'error' })
  }
}
</script>

<template>
  <header
    class="flex h-15.5 shrink-0 items-center gap-2.5 border-b border-default px-4"
  >
    <div class="grid min-w-0 flex-1">
      <span
        class="truncate text-[22px] leading-tight font-semibold text-highlighted"
        :title="avatarDetails?.name"
      >
        {{ avatarDetails?.name ?? 'No avatar yet' }}
      </span>
      <span class="truncate font-mono text-[11px] text-muted">
        {{ avatarDetails ? `${avatarDetails.id} · ${controlCount} controls` : 'Waiting for VRChat OSC' }}
      </span>
    </div>

    <StatusChip
      v-if="avatarDetails?.id"
      icon="i-lucide-lock"
      :label="activeProfile?.name ?? 'No profile'"
      :tone="activeProfile ? 'warning' : 'muted'"
      @click="router.push(activeProfile ? `/profiles/${activeProfile.id}` : '/profiles')"
    />

    <StatusBar v-if="loggedIn" />

    <UButton
      v-if="loggedIn"
      icon="i-lucide-share-2"
      color="neutral"
      variant="subtle"
      :disabled="!shareUrl"
      class="rounded-2xl"
      @click="copyShareLink"
    >
      <span class="max-lg:hidden">Share</span>
    </UButton>

    <button
      v-if="loggedIn && user"
      type="button"
      class="grid size-10.5 shrink-0 cursor-pointer place-items-center rounded-2xl glass transition-colors hover:bg-elevated"
      title="Account"
      @click="router.push('/settings/account')"
    >
      <UAvatar
        :src="user.discord.avatar"
        :alt="user.discord.name"
        size="sm"
      />
    </button>

    <UButton
      v-else
      :disabled="!authUrl"
      icon="ic:baseline-discord"
      class="rounded-2xl bg-[#5865f2] text-white hover:bg-[#4752c4]"
      @click="api.openUrl(authUrl!)"
    >
      Sign in
    </UButton>
  </header>
</template>
