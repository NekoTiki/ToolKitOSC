// Round-robin + a basic circuit breaker over whichever providers are currently configured - what
// resolveDefaultProvider() (see ./index.ts) uses to pick a provider for accounts without
// model-select permission, instead of a fixed preference order that always lands every such
// account on the same one provider.
interface ProviderHealth {
  consecutiveFailures: number
  // Epoch ms; 0 (or already past) = healthy. Set once consecutiveFailures crosses the threshold.
  unhealthyUntil: number
}

const FAILURE_THRESHOLD = 3
const COOLDOWN_MS = 2 * 60 * 1000

const health = new Map<string, ProviderHealth>()

function getHealth(providerId: string): ProviderHealth {
  let entry = health.get(providerId)

  if (!entry) {
    entry = { consecutiveFailures: 0, unhealthyUntil: 0 }
    health.set(providerId, entry)
  }

  return entry
}

// Call after any provider call fails, whether that provider was auto-selected or explicitly picked
// by an account with model-select permission - repeated failures (a bad model id, an outage, a
// sustained rate limit) take it out of the auto-select rotation below for a cooldown period,
// instead of new requests keep landing on something that's currently broken.
export function recordProviderFailure(providerId: string): void {
  const entry = getHealth(providerId)

  entry.consecutiveFailures++

  if (entry.consecutiveFailures >= FAILURE_THRESHOLD) {
    entry.unhealthyUntil = Date.now() + COOLDOWN_MS
  }
}

export function recordProviderSuccess(providerId: string): void {
  const entry = getHealth(providerId)

  entry.consecutiveFailures = 0
  entry.unhealthyUntil = 0
}

export function isProviderHealthy(providerId: string): boolean {
  const entry = health.get(providerId)

  return !entry || Date.now() >= entry.unhealthyUntil
}

// Shared rotation pointer, not per-provider - in-memory and reset on restart is fine, it's just
// where in the cycle the next pick starts, not state that needs to survive.
let cursor = 0

// Picks the next provider among `candidates` in rotation, skipping any currently unhealthy one -
// this is what makes it a load balancer rather than a static preference order: repeat calls spread
// across every available provider instead of always landing on the same one. Falls back to the
// full candidate list if every one of them is currently marked unhealthy, rather than refusing to
// pick anyone - a real attempt (and its resulting success/failure) is more useful than a hard stop.
export function pickNextProvider<T extends { id: string }>(candidates: T[]): T | undefined {
  const healthy = candidates.filter((c) => isProviderHealthy(c.id))
  const pool = healthy.length ? healthy : candidates

  if (!pool.length) return undefined

  const chosen = pool[cursor % pool.length]

  cursor++

  return chosen
}
