<script setup lang="ts">
import type { Attempt, LiveAttempt } from '~/types/aiAttempts'

// Fully self-contained: fetching, pagination, the failures-only filter, the live WS connection and
// its in-progress rows, error expand/collapse, icons - everything the "recent generations" log
// needs, shared verbatim between /dashboard/stats (global, no `discordId`) and the per-user
// drill-down page (see generationLog.ts's own discordId-optional queries for the same split
// server-side). A page embedding this only ever needs to react to the two emits below if it also
// keeps its own separate aggregate (see dashboard/stats.vue's chart totals) that a live event
// should update too - this component never needs to know that's happening.
const props = defineProps<{ discordId?: string }>()

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

// Matches the server's listRecentAttempts default (server/utils/ai/generationLog.ts) - used to
// trim the list back down to a page's worth after splicing in a live-pushed row.
const ATTEMPTS_PAGE_SIZE = 25

// useFetch (not adminFetch/onMounted) so this actually runs during SSR - the middleware only checks
// "logged in" (see middleware/admin.ts), the real admin check is this request's own response, and
// awaiting it here means the initial HTML an admin gets already has real rows in it instead of a
// guaranteed-empty shell that fills in after hydration.
const failuresOnly = ref(false)
const page = ref(1)
const {
  data: attemptsResponse,
  pending: attemptsLoading,
  error: attemptsError,
  refresh: refreshAttempts
} = await useFetch<{ attempts: Attempt[] }>('/api/admin/attempts', {
  query: computed(() => ({
    page: page.value,
    success: failuresOnly.value ? 'false' : undefined,
    discordId: props.discordId
  }))
})

if (attemptsError.value) await redirectIfUnauthorized(attemptsError.value)

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

  return Array.from(liveAttempts.value.values()).sort((a, b) => b.startedAt.localeCompare(a.startedAt))
})

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
  if (page.value === 1 && (!failuresOnly.value || !attempt.success)) {
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
  <UCard>
    <div class="mb-3 flex items-center justify-between gap-2">
      <p class="text-sm font-medium">
        Recent generations
      </p>
      <UCheckbox
        v-model="failuresOnly"
        label="Failures only"
      />
    </div>

    <div
      v-if="attemptsLoading"
      class="py-8 text-center text-sm text-muted"
    >
      Loading…
    </div>

    <div
      v-else-if="!attempts.length && !visibleLiveAttempts.length"
      class="py-8 text-center text-sm text-muted"
    >
      Nothing to show.
    </div>

    <div
      v-else
      class="flex flex-col divide-y divide-default text-sm"
    >
      <!-- Still running (see liveGenerations.ts) - swapped out for the real row the instant
      'generation-logged' arrives with a matching requestId (see applyGenerationLogged). -->
      <div
        v-for="attempt in visibleLiveAttempts"
        :key="attempt.requestId"
        class="flex flex-col gap-1.5 py-2"
      >
        <div class="flex flex-wrap items-center gap-2">
          <UBadge
            color="info"
            variant="subtle"
          >
            <span class="flex items-center gap-1">
              <span class="size-1.5 animate-pulse rounded-full bg-info" />
              Generating
            </span>
          </UBadge>
          <span class="font-medium">{{ attempt.displayName || attempt.discordId }}</span>
          <span class="text-muted">{{ attempt.avatarName }}</span>
          <span class="flex items-center gap-1 text-muted">
            <UIcon
              :name="providerIcon"
              class="size-3.5"
            />{{ attempt.providerLabel }}
          </span>
          <span class="text-muted">·</span>
          <span class="flex items-center gap-1 text-muted">
            <UIcon
              :name="profileIcon"
              class="size-3.5"
            />{{ attempt.profileLabel }}
          </span>
          <span class="ml-auto text-xs text-muted">
            {{ elapsedSeconds(attempt.startedAt).toFixed(1) }}s
          </span>
        </div>
      </div>

      <div
        v-for="attempt in attempts"
        :key="attempt.id"
        class="flex flex-col gap-1.5 py-2"
      >
        <div class="flex flex-wrap items-center gap-2">
          <!-- A dedicated toggle button, separate from the error text itself below - so the text
          is plain, selectable content, not something wrapped in an interactive element that would
          fight a click-drag selection or a double-click-to-select-word. -->
          <UButton
            v-if="attempt.errorMessage"
            :icon="expandedErrors[attempt.id] ? 'i-lucide-chevron-down' : 'i-lucide-chevron-right'"
            size="xs"
            color="neutral"
            variant="ghost"
            square
            @click="toggleError(attempt.id)"
          />
          <UBadge
            :color="attempt.success ? 'success' : 'error'"
            :icon="attempt.success ? successIcon : failureIcon(attempt.failureReason)"
            variant="subtle"
          >
            {{ attempt.success ? 'OK' : (attempt.failureReasonLabel ?? 'Error') }}
          </UBadge>
          <UBadge
            v-if="attempt.refunded"
            color="neutral"
            variant="subtle"
            :icon="refundedIcon"
          >
            Refunded
          </UBadge>
          <span class="font-medium">{{ attempt.displayName || attempt.discordId }}</span>
          <span class="text-muted">{{ attempt.avatarName }}</span>
          <span class="flex items-center gap-1 text-muted">
            <UIcon
              :name="providerIcon"
              class="size-3.5"
            />{{ attempt.providerLabel }}
          </span>
          <span class="text-muted">·</span>
          <span class="flex items-center gap-1 text-muted">
            <UIcon
              :name="profileIcon"
              class="size-3.5"
            />{{ attempt.profileLabel }}
          </span>
          <span class="ml-auto text-xs text-muted">
            <template v-if="attempt.durationMs !== null">{{ (attempt.durationMs / 1000).toFixed(1) }}s · </template>{{ new Date(attempt.createdAt).toLocaleString() }}
          </span>
        </div>

        <!-- Error message always gets its own line below the main row, instead of competing for
        space inline with the name/provider/timestamp - collapsed shows one truncated line at the
        row's full width; expanded (same toggle button above) grows into the full wrapped block.
        The expanded case is plain text, not a button, so it can be selected/copied normally. -->
        <button
          v-if="attempt.errorMessage && !expandedErrors[attempt.id]"
          type="button"
          class="ml-8 truncate text-left text-xs text-error"
          @click="toggleError(attempt.id)"
        >
          {{ attempt.errorMessage }}
        </button>
        <p
          v-if="attempt.errorMessage && expandedErrors[attempt.id]"
          class="ml-8 rounded-md bg-elevated p-2 text-xs whitespace-pre-wrap break-words text-error select-text"
        >
          {{ attempt.errorMessage }}
        </p>
      </div>
    </div>

    <div class="mt-3 flex items-center justify-end gap-2">
      <UButton
        size="xs"
        variant="soft"
        icon="i-lucide-chevron-left"
        :disabled="page <= 1"
        @click="page--"
      />
      <span class="text-xs text-muted">Page {{ page }}</span>
      <UButton
        size="xs"
        variant="soft"
        icon="i-lucide-chevron-right"
        :disabled="attempts.length < 25"
        @click="page++"
      />
    </div>
  </UCard>
</template>
