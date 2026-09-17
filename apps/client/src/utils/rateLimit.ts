// Fixed-window counter, per key - mirrors apps/server's own server/utils/rateLimiter.ts (same
// approach, separate implementation - one runs in Nitro, the other in this Vue app, nothing to
// share between them). See that file's comment for why a fixed window (not sliding) is good
// enough here.
//
// The threshold only has to clear a dragged slider (the one continuous, naturally high-rate
// interaction) - ControlSlider.vue debounces its own outgoing commands to ~1 per 100ms (~10/s)
// client-side already, so this doesn't need to tolerate anything higher than that plus some
// margin. Deliberately not throttled again before this check: a slider's displayed position
// depends on this exact round trip, so anything that buffers a command before executing it makes
// dragging feel laggy for whoever's holding it - the rate is kept down at the true source instead.
const WINDOW_MS = 1000
const DEFAULT_MAX_PER_WINDOW = 20

interface Bucket {
  count: number
  windowStart: number
}

const buckets = new Map<string, Bucket>()

export function isRateLimited(key: string, maxPerWindow = DEFAULT_MAX_PER_WINDOW): boolean {
  const now = Date.now()
  const bucket = buckets.get(key)

  if (!bucket || now - bucket.windowStart >= WINDOW_MS) {
    buckets.set(key, { count: 1, windowStart: now })
    return false
  }

  bucket.count += 1

  return bucket.count > maxPerWindow
}
