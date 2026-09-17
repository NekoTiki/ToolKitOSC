import { purgeOldCommands } from '@renderer/composables/useCommandsDb'
import { onBeforeUnmount, onMounted } from 'vue'

// This app can legitimately stay open for days at a time (see useTraySettings.ts's "keep running
// in the tray"), so a purge that only ran once at startup would miss every log that crosses the
// 7-day retention window mid-session. Six hours keeps that gap small without running the delete
// query needlessly often.
const PURGE_INTERVAL_MS = 6 * 60 * 60 * 1000

// Called once, unconditionally, from App.vue - deliberately not tied to sign-in/avatar state the
// way useCommandsDb.ts's old purge() accidentally was (it only ran via whichever component first
// called useCommandsDb(), which happened to be StatusBar.vue, itself gated behind `v-if="loggedIn"`).
export function useLogRetention(): void {
  let intervalId: ReturnType<typeof setInterval> | undefined

  onMounted(() => {
    void purgeOldCommands()

    intervalId = setInterval(() => void purgeOldCommands(), PURGE_INTERVAL_MS)
  })

  onBeforeUnmount(() => {
    if (intervalId) clearInterval(intervalId)
  })
}
