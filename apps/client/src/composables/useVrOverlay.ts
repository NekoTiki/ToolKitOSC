import { useLockedControls } from '@renderer/composables/useLockedControls'
import { useStopEverything } from '@renderer/composables/useStopEverything'
import { api } from '@renderer/lib/tauri-bridge'
import { watch } from 'vue'

// The wrist panel (src-tauri/src/wrist) runs in Rust, but Stop everything's pause and the profiles
// live here: this tells Rust the profiles, and carries out Stop, Resume and profile changes pressed
// on the panel. Called once, from App.vue.
export function useVrOverlay(): void {
  const { lockedControlGroups, currentLockedControlsGroup } = useLockedControls()
  const { stop, resume } = useStopEverything()

  watch(
    [() => lockedControlGroups.value.map(({ id, name }) => ({ id, name })), currentLockedControlsGroup],
    ([profiles, profileId]) => void api.overlaySetContext(profiles, profileId),
    { immediate: true, deep: true }
  )

  api.onOverlayAction((action) => {
    switch (action.type) {
      case 'stop':
        stop()
        break
      case 'resume':
        resume()
        break
      case 'profile':
        // A profile deleted since is ignored.
        if (action.id === null || lockedControlGroups.value.some((p) => p.id === action.id)) currentLockedControlsGroup.value = action.id
        break
    }
  })
}
