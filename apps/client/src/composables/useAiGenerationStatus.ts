import type { AiGenerateErrorMessage, AiGenerateProgressMessage, AiGenerateResultMessage, ControlGroup } from '@toolkitosc/shared-ui'
import { ref } from 'vue'

// A generation now finishes over the host WS connection (see useWebsocketHost.ts's
// 'ai-generate-*' handlers), not the POST /api/ai/suggest-controls response itself - that request
// only ever returns a requestId, immediately (see the server's suggest-controls.post.ts for why:
// a Cloudflare tunnel in front of it enforces a ~2.1 minute request timeout, well short of what a
// Heavy-profile generation can take). Module-scope, not per-caller state, same as useAiCredits -
// the WS push arrives independently of whichever useAiControlSuggestions() instance is waiting on
// it, so the two need a shared place to meet.
const pending = new Map<string, { resolve: (groups: ControlGroup[]) => void; reject: (error: Error) => void }>()

// Only the most recently started request's flavor text is shown - the AI page only ever
// has one generation in flight at a time (the Generate button is disabled while loading).
const activeRequestId = ref<string | null>(null)
const statusMessage = ref<string | null>(null)

// A generation that never gets a WS reply (host reconnects mid-request, the interval/result push
// gets lost, whatever) would otherwise leave the caller's promise - and the modal's loading state -
// hanging forever, despite the account having already been charged. Generous on purpose: real
// generations can legitimately take a couple of minutes now that they're not bounded by the
// Cloudflare request timeout anymore.
const WAIT_TIMEOUT_MS = 5 * 60 * 1000

export function useAiGenerationStatus(): {
  statusMessage: typeof statusMessage
  waitForResult: (requestId: string) => Promise<ControlGroup[]>
  applyProgress: (update: AiGenerateProgressMessage) => void
  applyResult: (update: AiGenerateResultMessage) => void
  applyError: (update: AiGenerateErrorMessage) => void
} {
  const settle = (requestId: string): void => {
    pending.delete(requestId)

    if (activeRequestId.value === requestId) {
      activeRequestId.value = null
      statusMessage.value = null
    }
  }

  const waitForResult = (requestId: string): Promise<ControlGroup[]> => {
    activeRequestId.value = requestId
    statusMessage.value = 'Thinking…'

    return new Promise<ControlGroup[]>((resolve, reject) => {
      const timeout = setTimeout(() => {
        pending.delete(requestId)
        reject(new Error("Timed out waiting for a response - it may still finish, check back shortly."))
      }, WAIT_TIMEOUT_MS)

      pending.set(requestId, {
        resolve: (groups) => {
          clearTimeout(timeout)
          resolve(groups)
        },
        reject: (error) => {
          clearTimeout(timeout)
          reject(error)
        }
      })
    })
  }

  const applyProgress = (update: AiGenerateProgressMessage): void => {
    if (update.requestId !== activeRequestId.value) return

    statusMessage.value = update.message
  }

  const applyResult = (update: AiGenerateResultMessage): void => {
    pending.get(update.requestId)?.resolve(update.groups)
    settle(update.requestId)
  }

  const applyError = (update: AiGenerateErrorMessage): void => {
    pending.get(update.requestId)?.reject(new Error(update.message))
    settle(update.requestId)
  }

  return { statusMessage, waitForResult, applyProgress, applyResult, applyError }
}
