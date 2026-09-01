import { api } from '@renderer/lib/tauri-bridge'
import { onMounted, ref } from 'vue'

import type { AvatarDetails } from '../env.d'

const avatarDetails = ref<AvatarDetails | null>(null)

export function useAvatarDetails(onAvatarDetails?: (details: AvatarDetails) => void): {
  avatarDetails: ReturnType<typeof ref<AvatarDetails | null>>
} {
  onMounted(() => {
    api.onAvatarDetails((details: AvatarDetails) => {
      avatarDetails.value = details
      if (onAvatarDetails) onAvatarDetails(details)
    })
  })

  return { avatarDetails }
}
