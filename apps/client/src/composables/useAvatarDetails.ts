import { api } from '@renderer/lib/tauri-bridge'
import { ref } from 'vue'

import type { AvatarDetails } from '../env.d'

const avatarDetails = ref<AvatarDetails | null>(null)
const changeCallbacks: Array<(details: AvatarDetails) => void> = []

// Registered at module scope, not inside onMounted, to match useOscMessages.ts's
// onOscMessage/onOscMessageBulk registration - `listen()` resolves the actual event subscription
// asynchronously, so registering it from a component's onMounted races App.vue's own
// onMounted(() => api.ready()): if ready()'s replay emit reaches the backend before this listener
// has actually finished registering, the event is silently dropped and the frontend is stuck
// showing "no avatar detected" even though the backend already knows the avatar (confirmed live -
// this is what was causing that exact symptom to persist after the app's own OSC-toggle
// workaround stopped helping, once avatar-change detection moved to a deduped OSCQuery poll with
// no natural second delivery to fall back on). Module-scope evaluation happens during Vue's
// initial script setup, before any component's onMounted can fire, so there's no race here.
api.onAvatarDetails((details: AvatarDetails) => {
  avatarDetails.value = details
  changeCallbacks.forEach((callback) => callback(details))
})

export function useAvatarDetails(onAvatarDetails?: (details: AvatarDetails) => void): {
  avatarDetails: ReturnType<typeof ref<AvatarDetails | null>>
} {
  if (onAvatarDetails) changeCallbacks.push(onAvatarDetails)

  return { avatarDetails }
}
