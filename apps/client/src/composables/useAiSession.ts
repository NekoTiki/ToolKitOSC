import type { AiOptions } from '@renderer/composables/useAiControlSuggestions'
import { useAiControlSuggestions } from '@renderer/composables/useAiControlSuggestions'
import { useAiCredits } from '@renderer/composables/useAiCredits'
import { useAvatarDetails } from '@renderer/composables/useAvatarDetails'
import { router } from '@renderer/router'
import type { ControlGroup } from '@vrc-osc-toolkit/shared-ui'
import type { Ref, WritableComputedRef } from 'vue'
import { computed, ref, watch } from 'vue'

// The AI page's state, kept at module level so a generation keeps going - and its result waits -
// while the user is on another page. Every run of this session keeps its full result, so the user
// can reopen an earlier one from the sidebar and pick more groups from it. Memory only: runs are
// gone when the app restarts.
export interface AiRun {
  id: number
  at: number
  profile: string
  // The avatar it was generated for - a run only applies to that avatar's parameters, so it can
  // only be reopened while that avatar is loaded.
  avatarId: string
  avatarName: string
  groups: ControlGroup[]
  controls: number
  // Indexes of the groups already added to the controls, so reopening a run doesn't offer them as
  // if they were new.
  added: Set<number>
}

const options = ref<AiOptions | null>(null)
const optionsError = ref<string | null>(null)
const optionsLoading = ref(false)
// The run being reviewed on the page, if any.
const openRunId = ref<number | null>(null)
let nextRunId = 1
const runError = ref<string | null>(null)
const generating = ref(false)
const runs = ref<AiRun[]>([])
// How the last run ended, when it ended while the user was on another page - the rail shows it
// until they come back to the AI page.
const unseen = ref<'success' | 'error' | null>(null)

let suggestions: ReturnType<typeof useAiControlSuggestions> | undefined
let creditsWatched = false

const isAiPage = (path: string): boolean => path.startsWith('/ai')

const notifyAway = (): void => {
  const failed = !!runError.value

  unseen.value = failed ? 'error' : 'success'
  useToast().add({
    title: failed ? 'AI generation failed' : 'AI suggestions are ready',
    description: failed ? (runError.value ?? undefined) : `${runs.value[0]?.groups.length ?? 0} groups to review`,
    icon: failed ? 'i-lucide-circle-alert' : 'i-lucide-sparkles',
    color: failed ? 'error' : 'success',
    actions: [{ label: 'Open', onClick: () => void router.push('/ai') }]
  })
}

export function useAiSession(): {
  options: Ref<AiOptions | null>
  optionsError: Ref<string | null>
  optionsLoading: Ref<boolean>
  result: WritableComputedRef<ControlGroup[] | null>
  openRun: Ref<AiRun | null>
  openRunById: (id: number) => void
  markAdded: (indexes: number[]) => void
  runError: Ref<string | null>
  generating: Ref<boolean>
  runs: Ref<AiRun[]>
  unseen: Ref<'success' | 'error' | null>
  loadOptions: () => Promise<void>
  run: (provider: string | undefined, profile: string, profileLabel: string) => Promise<void>
} {
  suggestions ??= useAiControlSuggestions()

  const { avatarDetails } = useAvatarDetails()

  const openRun = computed(() => runs.value.find((entry) => entry.id === openRunId.value) ?? null)

  // The open run's groups; setting it to null closes the run (it stays in the sidebar).
  const result = computed<ControlGroup[] | null>({
    get: () => openRun.value?.groups ?? null,
    set: (value) => {
      if (value === null) openRunId.value = null
    }
  })

  const openRunById = (id: number): void => {
    const entry = runs.value.find((r) => r.id === id)

    if (entry && entry.avatarId === avatarDetails.value?.id) openRunId.value = id
  }

  const markAdded = (indexes: number[]): void => {
    if (!openRun.value) return

    openRun.value.added = new Set([...openRun.value.added, ...indexes])
  }

  // Credits can change while the page is open (another run, an admin bonus) - pushed over WS.
  if (!creditsWatched) {
    creditsWatched = true
    const { latest } = useAiCredits()

    watch(latest, (update) => {
      if (update && options.value) options.value = { ...options.value, remainingCredits: update.remainingCredits }
    })

    watch(
      () => router.currentRoute.value.path,
      (path) => {
        if (isAiPage(path)) unseen.value = null
      }
    )

    // A run's groups only fit the avatar it was made for: close it when another avatar loads.
    watch(
      () => useAvatarDetails().avatarDetails.value?.id,
      (id) => {
        const open = runs.value.find((entry) => entry.id === openRunId.value)

        if (open && open.avatarId !== id) openRunId.value = null
      }
    )
  }

  const loadOptions = async (): Promise<void> => {
    optionsLoading.value = true

    try {
      options.value = await suggestions!.listOptions()
      optionsError.value = null
    } catch (err) {
      optionsError.value = err instanceof Error ? err.message : String(err)
    } finally {
      optionsLoading.value = false
    }
  }

  const run = async (provider: string | undefined, profile: string, profileLabel: string): Promise<void> => {
    generating.value = true
    openRunId.value = null
    runError.value = null

    // Captured now: the avatar the parameters were sent for, even if it changes mid-generation.
    const avatar = avatarDetails.value

    try {
      const groups = await suggestions!.generate(provider, profile)
      const id = nextRunId++

      runs.value.unshift({
        id,
        at: Date.now(),
        profile: profileLabel,
        avatarId: avatar?.id ?? '',
        avatarName: avatar?.name ?? 'Unknown avatar',
        groups,
        controls: groups.reduce((n, g) => n + g.controls.length, 0),
        added: new Set()
      })
      openRunId.value = id
    } catch (err) {
      runError.value = err instanceof Error ? err.message : String(err)
    } finally {
      generating.value = false
      void loadOptions()
      if (!isAiPage(router.currentRoute.value.path)) notifyAway()
    }
  }

  return { options, optionsError, optionsLoading, result, openRun, openRunById, markAdded, runError, generating, runs, unseen, loadOptions, run }
}
