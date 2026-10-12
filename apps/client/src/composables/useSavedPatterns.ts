import { getUUID } from '@renderer/utils/uuid'
import type { Ref } from 'vue'
import { ref } from 'vue'

// Toy patterns the host recorded on the Patterns page (see PatternRecorder.vue). They play next to
// the built-in ones (see getPattern in useIntifacePatterns.ts). Global, not per avatar: a pattern is
// about the toy, not the avatar.
export interface SavedPattern {
  id: string
  name: string
  icon?: string
  // Intensities 0-100, one every PATTERN_SAMPLE_MS. Playback loops them.
  samples: number[]
  updatedAt: number
}

export const PATTERN_SAMPLE_MS = 50
export const PATTERN_MAX_MS = 30_000
// Anything shorter is thrown away: it's almost always a slip, not a pattern.
export const PATTERN_MIN_MS = 1000

const STORAGE_KEY = 'savedPatterns'

const load = (): SavedPattern[] => {
  try {
    const saved = JSON.parse(localStorage.getItem(STORAGE_KEY) ?? '[]')

    return Array.isArray(saved) ? saved.filter((pattern) => Array.isArray(pattern?.samples) && pattern.samples.length) : []
  } catch {
    return []
  }
}

const savedPatterns = ref<SavedPattern[]>(load())

const save = (): void => localStorage.setItem(STORAGE_KEY, JSON.stringify(savedPatterns.value))

const getSavedPattern = (id: string): SavedPattern | undefined => savedPatterns.value.find((pattern) => pattern.id === id)

const addPattern = (pattern: Pick<SavedPattern, 'name' | 'icon' | 'samples'>): SavedPattern => {
  const created: SavedPattern = { ...pattern, id: getUUID(), updatedAt: Date.now() }

  savedPatterns.value = [...savedPatterns.value, created]
  save()

  return created
}

const updatePattern = (id: string, changes: Partial<Pick<SavedPattern, 'name' | 'icon' | 'samples'>>): void => {
  savedPatterns.value = savedPatterns.value.map((pattern) => (pattern.id === id ? { ...pattern, ...changes, updatedAt: Date.now() } : pattern))
  save()
}

const deletePattern = (id: string): void => {
  savedPatterns.value = savedPatterns.value.filter((pattern) => pattern.id !== id)
  save()
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
