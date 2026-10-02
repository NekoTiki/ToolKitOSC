import type { Subscription } from 'dexie'
import { liveQuery } from 'dexie'
import type { Ref, WatchSource } from 'vue'
import { onScopeDispose, shallowRef, watch } from 'vue'

// A Dexie liveQuery that also re-runs when Vue state it depends on changes. liveQuery alone only
// re-runs on writes to the tables it read; here `source` changes (a selected viewer, a filter)
// rebuild the subscription too.
export function useLiveQuery<T, S>(source: WatchSource<S>, query: (value: S) => Promise<T>, initial: T): Ref<T> {
  const result = shallowRef<T>(initial)
  let subscription: Subscription | undefined

  watch(
    source,
    (value) => {
      subscription?.unsubscribe()
      subscription = liveQuery(() => query(value)).subscribe({
        next: (next) => (result.value = next),
        error: (error) => console.error('Live query failed', error)
      })
    },
    { immediate: true, deep: true }
  )

  onScopeDispose(() => subscription?.unsubscribe())

  return result
}
