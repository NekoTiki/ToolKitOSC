import type { Credits } from '@renderer/lib/credits'
import type { Ref } from 'vue'
import { ref } from 'vue'

// The libraries list for Settings › About › Credits, written next to the page at build time (see
// vite-plugin-credits.ts). Fetched the first time the page opens, then kept for the session.
const credits = ref<Credits | null>(null)
const loading = ref(false)
const error = ref(false)

async function load(): Promise<void> {
  if (credits.value || loading.value) return
  loading.value = true
  error.value = false
  try {
    const response = await fetch('/credits.json')
    if (!response.ok) throw new Error(`HTTP ${response.status}`)
    credits.value = (await response.json()) as Credits
  } catch (e) {
    console.error('Failed to load credits', e)
    error.value = true
  } finally {
    loading.value = false
  }
}

export function useCredits(): {
  credits: Ref<Credits | null>
  loading: Ref<boolean>
  error: Ref<boolean>
  load: () => Promise<void>
} {
  return { credits, loading, error, load }
}
