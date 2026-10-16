import { api } from '@renderer/lib/tauri-bridge'
import { getUUID } from '@renderer/utils/uuid'
import type { Ref } from 'vue'
import { ref } from 'vue'

// Toy patterns the host recorded on the Patterns page (see PatternRecorder.vue). They play next to
// the built-in ones (see getPattern in useIntifacePatterns.ts). Global, not per avatar: a pattern is
// about the toy, not the avatar. Rust keeps them in patterns.json (src-tauri/src/toys/saved.rs), so
// they play while the window is closed.
export interface SavedPattern {
  id: string
  name: string
  icon?: string
  // Intensities 0-100, one every PATTERN_SAMPLE_MS. Playback loops them.
  samples: number[]
  updatedAt: number
}

export { PATTERN_MAX_MS, PATTERN_MIN_MS, PATTERN_SAMPLE_MS } from '@toolkitosc/shared-ui'

// Where the webview kept them before Rust took over. Handed over once, then removed.
const LEGACY_STORAGE_KEY = 'savedPatterns'

const savedPatterns = ref<SavedPattern[]>([])

api.onSavedPatterns((patterns) => (savedPatterns.value = patterns))

const legacy = localStorage.getItem(LEGACY_STORAGE_KEY)

if (legacy !== null) {
  let patterns: SavedPattern[] = []

  try {
    const parsed = JSON.parse(legacy)

    patterns = Array.isArray(parsed) ? parsed.filter((pattern) => Array.isArray(pattern?.samples) && pattern.samples.length) : []
  } catch {
    // Unreadable: nothing to keep.
  }

  void api
    .patternsMigrate(patterns)
    .then(() => localStorage.removeItem(LEGACY_STORAGE_KEY))
    .catch((error) => console.error('Could not move the recorded patterns to the app:', error))
    .finally(() => void api.patternsSavedList().then((list) => (savedPatterns.value = list)))
} else {
  void api.patternsSavedList().then((list) => (savedPatterns.value = list))
}

const getSavedPattern = (id: string): SavedPattern | undefined => savedPatterns.value.find((pattern) => pattern.id === id)

// Shown straight away; Rust saves it and sends the list back.
const store = (pattern: SavedPattern): void => {
  const known = savedPatterns.value.some((existing) => existing.id === pattern.id)

  savedPatterns.value = known ? savedPatterns.value.map((existing) => (existing.id === pattern.id ? pattern : existing)) : [...savedPatterns.value, pattern]
  void api.patternsSave(pattern)
}

const addPattern = (pattern: Pick<SavedPattern, 'name' | 'icon' | 'samples'>): SavedPattern => {
  const created: SavedPattern = { ...pattern, id: getUUID(), updatedAt: Date.now() }

  store(created)

  return created
}

const updatePattern = (id: string, changes: Partial<Pick<SavedPattern, 'name' | 'icon' | 'samples'>>): void => {
  const pattern = getSavedPattern(id)

  if (pattern) store({ ...pattern, ...changes, updatedAt: Date.now() })
}

const deletePattern = (id: string): void => {
  savedPatterns.value = savedPatterns.value.filter((pattern) => pattern.id !== id)
  void api.patternsDelete(id)
}

export function useSavedPatterns(): {
  savedPatterns: Ref<SavedPattern[]>
  getSavedPattern: (id: string) => SavedPattern | undefined
  addPattern: (pattern: Pick<SavedPattern, 'name' | 'icon' | 'samples'>) => SavedPattern
  updatePattern: (id: string, changes: Partial<Pick<SavedPattern, 'name' | 'icon' | 'samples'>>) => void
  deletePattern: (id: string) => void
} {
  return { savedPatterns, getSavedPattern, addPattern, updatePattern, deletePattern }
}
