// Fixed-window counter, per key - cheap and good enough here: what matters is catching a client
// sending far more messages than any real interaction would, not smoothing bursts perfectly. The
// window resets wholesale on expiry rather than sliding, so a client could in principle send up to
// 2x the limit right across a window boundary; that's an acceptable trade for not tracking a
// timestamp per message.
//
// The threshold only has to clear a dragged slider (the one continuous, naturally high-rate
// interaction) - ControlSlider.vue debounces its own outgoing commands to ~1 per 100ms (~10/s)
// client-side already, so this doesn't need to tolerate anything higher than that plus some
// margin. Deliberately not throttled again on this end of the wire: a slider's displayed position
// depends on this exact round trip, so anything that buffers a message before forwarding it makes
// dragging feel laggy for whoever's holding it - the rate is kept down at the true source instead.
const WINDOW_MS = 1000
const DEFAULT_MAX_PER_WINDOW = 20
// Logged-in users are identifiable (banning/abuse follow-up can target their Discord id rather
// than just an IP), so they're worth trusting with more headroom than an anonymous guest.
export const AUTHENTICATED_MAX_PER_WINDOW = 50

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

// Called on disconnect (see ws/[ws].ts's close handler) - without this, a bucket for every
// session id that's ever connected would sit in memory for as long as the process runs.
export function clearRateLimit(key: string): void {
  buckets.delete(key)
}
