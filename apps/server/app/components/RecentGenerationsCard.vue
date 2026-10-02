<script setup lang="ts">
import type { Attempt, LiveAttempt } from '~/types/aiAttempts'

// Fully self-contained: fetching, pagination, the failure/provider filters, the live WS connection
// and its in-progress rows, error expand/collapse, icons - everything the generations log needs.
// Two modes: a short preview with a "See all" link (Overview and a user's page), and the full log
// with filters and paging (/dashboard/generations). `discordId` narrows either to one account (see
// generationLog.ts's own discordId-optional queries for the same split server-side). A page
// embedding this only ever needs to react to the emits below if it also keeps its own separate
// aggregate (see dashboard/stats.vue's chart totals) that a live event should update too.
const props = withDefaults(defineProps<{ discordId?: string; full?: boolean; limit?: number; userName?: string }>(), {
  discordId: undefined,
  full: false,
  limit: 6,
  userName: undefined
})

const emit = defineEmits<{
  (e: 'generation-logged', attempt: Attempt): void
  (e: 'generation-started', attempt: LiveAttempt): void
  // Fired whenever the live WS (re)connects, including the very first connect - a page with its
  // own separate data (e.g. dashboard/stats.vue's AI/control stats) can use this as its cue to
  // reload that too, the same moment this card recovers anything it might have missed.
  (e: 'reconnected'): void
}>()

const { adminFetch, redirectIfUnauthorized } = useAdminApi()
const { successIcon, failureIcon, refundedIcon, providerIcon, profileIcon } = useAiLogIcons()

const seeAllLink = computed(() => (props.discordId ? `/dashboard/generations?user=${props.discordId}` : '/dashboard/generations'))

// Matches the server's listRecentAttempts default (server/utils/ai/generationLog.ts) - used to
// trim the list back down to a page's worth after splicing in a live-pushed row.
const ATTEMPTS_PAGE_SIZE = 25

// useFetch (not adminFetch/onMounted) so this actually runs during SSR - the middleware only checks
// "logged in" (see middleware/admin.ts), the real admin check is this request's own response, and
// awaiting it here means the initial HTML an admin gets already has real rows in it instead of a
// guaranteed-empty shell that fills in after hydration.
const failuresOnly = ref(false)
const provider = ref<string | null>(null)
const page = ref(1)

watch([failuresOnly, provider], () => (page.value = 1))
const {
  data: attemptsResponse,
  pending: attemptsLoading,
  error: attemptsError,
  refresh: refreshAttempts
} = await useFetch<{ attempts: Attempt[] }>('/api/admin/attempts', {
  query: computed(() => ({
    page: page.value,
    success: failuresOnly.value ? 'false' : undefined,
    provider: provider.value ?? undefined,
    discordId: props.discordId
  }))
})

if (attemptsError.value) await redirectIfUnauthorized(attemptsError.value)

// The provider filter's choices - only needed by the full log.
const { data: providersResponse } = await useFetch<{ providers: { id: string; label: string }[] }>('/api/admin/providers', {
  immediate: props.full
})

// A plain, freely-mutable copy, not attemptsResponse itself - the WS live-update splicing below
// (applyGenerationLogged) needs to add a row without that then getting clobbered the next time
// attemptsResponse's own reactive query changes and refetches from the server.
const attempts = ref<Attempt[]>([])

watch(attemptsResponse, (res) => (attempts.value = res?.attempts ?? []), { immediate: true })

// useFetch here too, for the same SSR reason as attempts above - a page loaded (or the WS below
// reconnected) while a generation is already minutes into running should still be able to show it
// as live, not just from the next one that starts. GET /api/admin/live-attempts isn't itself
// discordId-filterable (it's a handful of in-memory entries at most - see liveGenerations.ts), so
// that scoping happens here instead, the same place applyGenerationStarted below does it for the
// live WS push.
const {
  data: liveAttemptsResponse,
  error: liveAttemptsError,
  refresh: refreshLiveAttempts
} = await useFetch<{ attempts: LiveAttempt[] }>('/api/admin/live-attempts')

if (liveAttemptsError.value) await redirectIfUnauthorized(liveAttemptsError.value)

const liveAttempts = ref<Map<string, LiveAttempt>>(new Map())

function inScope(discordId: string): boolean {
  return !props.discordId || discordId === props.discordId
}

const matchesFilters = (attempt: { provider: string | null; success?: boolean }): boolean =>
  (!provider.value || attempt.provider === provider.value) && (!failuresOnly.value || attempt.success === false)

// Keyed by requestId, not an array - both this and applyGenerationLogged (remove, once the real
// row replaces it) below need direct lookup. Only ever adds/updates entries here, never removes
// ones missing from a given response: that can race a 'generation-started' WS push that already
// landed locally for an even-newer request the snapshot predates, and any entry that's genuinely
// gone stale (finished between the snapshot and now) is cleaned up by its own 'generation-logged'
// push regardless, which always eventually arrives.
watch(
  liveAttemptsResponse,
  (res) => {
    for (const attempt of res?.attempts ?? []) {
      if (inScope(attempt.discordId)) liveAttempts.value.set(attempt.requestId, attempt)
    }
  },
  { immediate: true }
)

// Drives every live row's ticking duration (see the template below) - one shared interval for all
// of them rather than one per row. Simplicity over micro-optimizing an otherwise idle interval:
// this is a low-traffic admin-only view, and pausing/resuming the interval based on whether any
// live rows currently exist isn't worth the extra state for what's a single setInterval either way.
const now = ref(Date.now())
let nowTimer: ReturnType<typeof setInterval> | undefined

onMounted(() => {
  nowTimer = setInterval(() => (now.value = Date.now()), 500)
})
onBeforeUnmount(() => clearInterval(nowTimer))

function elapsedSeconds(startedAt: string): number {
  return Math.max(0, (now.value - new Date(startedAt).getTime()) / 1000)
}

// Live rows only make sense on page 1 (they're "right now", not a specific historical page) and
// while not filtering to failures only - a still-running attempt hasn't failed (or succeeded) yet,
// so it doesn't belong in that view. Newest-first, matching the finished list below.
const visibleLiveAttempts = computed(() => {
  if (page.value !== 1 || failuresOnly.value) return []

  return Array.from(liveAttempts.value.values())
    .filter(matchesFilters)
    .sort((a, b) => b.startedAt.localeCompare(a.startedAt))
})

// The preview shows running generations first, then the newest finished ones, `limit` rows in all.
const visibleLive = computed(() => (props.full ? visibleLiveAttempts.value : visibleLiveAttempts.value.slice(0, props.limit)))
const visibleAttempts = computed(() =>
  props.full ? attempts.value : attempts.value.slice(0, Math.max(0, props.limit - visibleLive.value.length))
)

// Live updates: /admin/ws pushes the actual new row whenever a generation attempt is logged (see
// the server's generationLog.ts) - applied locally below instead of re-running the REST fetches
// above on every single event, which would mean a full list re-fetch for what's usually just one
// more row.
interface GenerationLoggedEvent {
  type: 'generation-logged'
  attempt: Attempt
}

interface GenerationStartedEvent {
  type: 'generation-started'
  attempt: LiveAttempt
}

function applyGenerationStarted(attempt: LiveAttempt): void {
  if (!inScope(attempt.discordId)) return

  liveAttempts.value.set(attempt.requestId, attempt)
  emit('generation-started', attempt)
}

function applyGenerationLogged(attempt: Attempt): void {
  // The real, finished row takes over from here - see the watch(liveAttemptsResponse) above for
  // why this is the only place a live placeholder ever gets removed. Unconditional (not gated by
  // inScope): a no-op if this card never had that requestId in the first place.
  if (attempt.requestId) liveAttempts.value.delete(attempt.requestId)

  if (!inScope(attempt.discordId)) return

  // Only meaningful to splice into the visible list when looking at the first page (where a new
  // row would actually land) and it matches the current filter - otherwise it's left alone; the
  // admin will see it by paging back to 1, and the reconnect reload below catches up regardless.
  if (page.value === 1 && matchesFilters(attempt)) {
    attempts.value = [attempt, ...attempts.value].slice(0, ATTEMPTS_PAGE_SIZE)
  }

  emit('generation-logged', attempt)
}

let ws: WebSocket | null = null
let reconnectTimeout: ReturnType<typeof setTimeout> | undefined

async function connectWs(): Promise<void> {
  try {
    // The WS upgrade itself can't go through requireUserSession the way this fetch does, so this
    // ticket - minted over an authenticated REST call - is the actual auth check (see the server's
    // adminWsTickets.ts).
    const { ticket } = await adminFetch<{ ticket: string }>('/api/admin/ws-ticket')
    const protocol = location.protocol === 'https:' ? 'wss:' : 'ws:'

    ws = new WebSocket(`${protocol}//${location.host}/admin/ws?ticket=${ticket}`)

    // Catches up on anything missed while disconnected - a real reload, not the incremental path
    // above, since there's no way to know what was missed. Runs on the very first connect too
    // (harmless, just one extra fetch layered on top of the useFetch calls above, which already
    // ran once during SSR/on mount).
    ws.addEventListener('open', () => {
      void refreshAttempts()
      void refreshLiveAttempts()
      emit('reconnected')
    })

    ws.addEventListener('message', (event: MessageEvent<string>) => {
      try {
        const data = JSON.parse(event.data) as GenerationLoggedEvent | GenerationStartedEvent

        if (data.type === 'generation-logged') applyGenerationLogged(data.attempt)
        else if (data.type === 'generation-started') applyGenerationStarted(data.attempt)
      } catch (err) {
        console.error('[admin-ws] failed to apply message:', err)
      }
    })

    ws.addEventListener('close', () => {
      ws = null
      // Best-effort reconnect - a dashboard tab left open shouldn't need a manual reload just
      // because the connection dropped once (a server restart, a network blip).
      reconnectTimeout = setTimeout(connectWs, 5000)
    })
  } catch {
    reconnectTimeout = setTimeout(connectWs, 5000)
  }
}

onMounted(connectWs)
onBeforeUnmount(() => {
  clearTimeout(reconnectTimeout)
  ws?.close()
  ws = null
})

// Which attempt rows have their error message expanded - keyed by attempt id, reset on navigation
// (not persisted), so re-opening the page always starts collapsed.
const expandedErrors = ref<Record<number, boolean>>({})

function toggleError(id: number): void {
  expandedErrors.value[id] = !expandedErrors.value[id]
}
</script>

<template>
  <div class="grid gap-3.5">
    <div
      v-if="full"
      class="flex flex-wrap items-center gap-2"
    >
      <UButton
        icon="i-lucide-circle-x"
        :color="failuresOnly ? 'primary' : 'neutral'"
        :variant="failuresOnly ? 'soft' : 'outline'"
        class="rounded-full"
        :aria-pressed="failuresOnly"
        @click="failuresOnly = !failuresOnly"
      >
        Failures only
      </UButton>
      <span class="mx-1 h-7 w-px bg-(--ui-border)" />
      <UButton
        v-for="option in [{ id: null, label: 'All providers' }, ...(providersResponse?.providers ?? [])]"
        :key="option.id ?? 'all'"
        :color="provider === option.id ? 'primary' : 'neutral'"
        :variant="provider === option.id ? 'soft' : 'outline'"
        class="rounded-full"
        :aria-pressed="provider === option.id"
        @click="provider = option.id"
      >
        {{ option.label }}
      </UButton>
      <UButton
        v-if="discordId"
        to="/dashboard/generations"
        color="primary"
        variant="soft"
        trailing-icon="i-lucide-x"
        class="rounded-full"
        :aria-label="`Show everyone, not just ${userName ?? discordId}`"
      >
        {{ userName ?? discordId }}
      </UButton>
    </div>

    <UCard :ui="{ body: 'p-0 sm:p-0' }">
      <div
        v-if="!full"
        class="flex items-center gap-2.5 px-4.5 pt-4 pb-2"
      >
        <h2 class="flex-1 text-[17px] font-semibold text-highlighted">
          Recent generations
        </h2>
        <UButton
          :to="seeAllLink"
          size="md"
          color="neutral"
          variant="outline"
        >
          See all
        </UButton>
      </div>

      <p
        v-if="attemptsLoading && !attempts.length"
        class="py-10 text-center text-sm text-muted"
      >
        Loading…
      </p>

      <p
        v-else-if="!visibleAttempts.length && !visibleLive.length"
        class="py-10 text-center text-sm text-muted"
      >
        {{ full && (failuresOnly || provider) ? 'Nothing matches these filters.' : 'No generations yet.' }}
      </p>

      <div
        v-else
        class="divide-y divide-default"
      >
        <!-- Still running (see liveGenerations.ts) - swapped out for the real row the instant
        'generation-logged' arrives with a matching requestId (see applyGenerationLogged). -->
        <div
          v-for="attempt in visibleLive"
          :key="attempt.requestId"
          class="grid min-h-16 grid-cols-[auto_minmax(0,1fr)] items-center gap-x-3 gap-y-1 px-3.5 py-3 sm:grid-cols-[auto_minmax(0,1fr)_auto]"
        >
          <UBadge
            color="primary"
            variant="subtle"
            class="rounded-full"
          >
            <span class="size-1.75 animate-pulse rounded-full bg-primary" />
            Generating
          </UBadge>
          <div class="min-w-0">
            <b class="text-sm font-medium text-highlighted">{{ attempt.displayName || attempt.discordId }}</b>
            <span class="text-sm text-muted"> · {{ attempt.avatarName }}</span>
            <small class="mt-0.5 flex items-center gap-1 truncate text-xs text-muted">
              <UIcon
                :name="providerIcon"
                class="size-3.5 shrink-0"
              />{{ attempt.providerLabel }} · {{ attempt.model }} ·
              <UIcon
                :name="profileIcon"
                class="size-3.5 shrink-0"
              />{{ attempt.profileLabel }}
            </small>
          </div>
          <span class="font-mono text-xs text-muted tabular-nums max-sm:col-start-2">{{ elapsedSeconds(attempt.startedAt).toFixed(1) }}s</span>
        </div>

        <div
          v-for="attempt in visibleAttempts"
          :key="attempt.id"
          class="grid min-h-16 grid-cols-[auto_minmax(0,1fr)] items-center gap-x-3 gap-y-1 px-3.5 py-3 sm:grid-cols-[auto_minmax(0,1fr)_auto]"
        >
          <UBadge
            :color="attempt.success ? 'success' : 'error'"
            :icon="attempt.success ? successIcon : failureIcon(attempt.failureReason)"
            variant="subtle"
            class="rounded-full"
          >
            {{ attempt.success ? 'OK' : (attempt.failureReasonLabel ?? 'Error') }}
          </UBadge>
          <div class="min-w-0">
            <b class="text-sm font-medium text-highlighted">{{ attempt.displayName || attempt.discordId }}</b>
            <span
              v-if="attempt.avatarName"
              class="text-sm text-muted"
            > · {{ attempt.avatarName }}</span>
            <UBadge
              v-if="attempt.refunded"
              color="neutral"
              variant="subtle"
              size="sm"
              :icon="refundedIcon"
              class="ml-1.5 rounded-full align-middle"
            >
              Refunded
            </UBadge>
            <small class="mt-0.5 flex items-center gap-1 truncate text-xs text-muted">
              <UIcon
                :name="providerIcon"
                class="size-3.5 shrink-0"
              />{{ attempt.providerLabel ?? 'No provider' }}<template v-if="attempt.model"> · {{ attempt.model }}</template><template v-if="attempt.profileLabel"> ·
                <UIcon
                  :name="profileIcon"
                  class="size-3.5 shrink-0"
                />{{ attempt.profileLabel }}</template>
            </small>
          </div>
          <div class="grid justify-items-end gap-1 font-mono text-xs whitespace-nowrap text-muted max-sm:col-start-2 max-sm:flex max-sm:items-center max-sm:justify-between">
            <span>
              <template v-if="attempt.durationMs !== null">{{ (attempt.durationMs / 1000).toFixed(1) }}s · </template><NuxtTime
                :datetime="attempt.createdAt"
                relative
              />
            </span>
            <!-- A separate toggle, so the error text itself stays plain, selectable content. -->
            <button
              v-if="attempt.errorMessage"
              type="button"
              class="-mr-1.5 flex h-8 cursor-pointer items-center gap-1 rounded-lg px-1.5 text-error hover:bg-error/10"
              :aria-expanded="!!expandedErrors[attempt.id]"
              @click="toggleError(attempt.id)"
            >
              <UIcon
                :name="expandedErrors[attempt.id] ? 'i-lucide-chevron-down' : 'i-lucide-chevron-right'"
                class="size-3.5"
              />{{ expandedErrors[attempt.id] ? 'Hide error' : 'Show error' }}
            </button>
          </div>
          <p
            v-if="attempt.errorMessage && expandedErrors[attempt.id]"
            class="col-span-full rounded-md bg-error/8 px-3 py-2.5 font-mono text-xs break-words whitespace-pre-wrap text-error select-text"
          >
            {{ attempt.errorMessage }}
          </p>
        </div>
      </div>
    </UCard>

    <div
      v-if="full"
      class="flex flex-wrap items-center gap-2"
    >
      <span class="flex-1 text-[13px] text-muted">Page {{ page }}</span>
      <UButton
        size="md"
        color="neutral"
        variant="outline"
        icon="i-lucide-chevron-left"
        :disabled="page <= 1"
        @click="page--"
      >
        Newer
      </UButton>
      <UButton
        size="md"
        color="neutral"
        variant="outline"
        trailing-icon="i-lucide-chevron-right"
        :disabled="attempts.length < ATTEMPTS_PAGE_SIZE"
        @click="page++"
      >
        Older
      </UButton>
    </div>
  </div>
</template>
