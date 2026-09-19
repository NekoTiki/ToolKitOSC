import { check } from '@tauri-apps/plugin-updater'
import type { Ref } from 'vue'
import { onMounted, ref } from 'vue'

// Guards the silent launch check below so it only ever runs once app-wide. SettingsModal.vue's
// overlay is instantiated eagerly (see useSettingsModal.ts), so its own useAppUpdater() call
// mounts around the same time as App.vue's - without this guard both fire their own check(),
// and both then toast.add() an "Update available" toast with the same id, landing two entries
// with the same key in the toasts array that Vue has to reconcile - which looks exactly like the
// toast flashing/closing instantly.
let hasCheckedOnLaunch = false

// Called both unconditionally from App.vue (silent - see useLogRetention.ts for the same
// call-once-on-mount pattern) and on demand from SettingsModal.vue's "Check for Updates" button
// (not silent - that click deserves feedback even when there's nothing to install).
export function useAppUpdater(): {
  checking: Ref<boolean>
  checkForUpdates: (options?: { silent?: boolean }) => Promise<void>
} {
  const toast = useToast()
  const checking = ref(false)

  const showUpdateToast = (update: NonNullable<Awaited<ReturnType<typeof check>>>): void => {
    const availableToastId = `app-update-${update.version}`

    toast.add({
      id: availableToastId,
      title: `Update available: v${update.version}`,
      description: 'Restarting will download and install the update.',
      icon: 'i-lucide-download',
      color: 'info',
      duration: 0,
      actions: [
        {
          label: 'Update & Restart',
          color: 'info',
          onClick: () => {
            // Every state below gets its own fresh toast id instead of reusing/merging one -
            // reusing an id (via add()'s mergeDuplicate or update()) kept closing the toast out
            // from under these transitions, so each message now gets its own component instance
            // rather than inheriting whatever internal state the previous one left behind.
            toast.remove(availableToastId)

            // Tracks whichever of the toasts below is currently showing, so each step can remove
            // exactly that one before adding the next - downloadAndInstall can still reject after
            // its 'Finished' event already fired (e.g. a bad signature is only caught once the
            // full download's in hand), so the catch below can't assume it's cleaning up the
            // "downloading" toast specifically.
            let currentToastId = `app-update-progress-${Date.now()}`

            toast.add({
              id: currentToastId,
              title: 'Downloading update…',
              icon: 'i-lucide-download',
              color: 'info',
              duration: 0
            })

            // On Windows the installer exits and relaunches the app itself once installed, so
            // there's nothing left to do here if that succeeds.
            update
              .downloadAndInstall((event) => {
                if (event.event === 'Finished') {
                  toast.remove(currentToastId)
                  currentToastId = `app-update-installing-${Date.now()}`
                  toast.add({
                    id: currentToastId,
                    title: 'Installing update…',
                    icon: 'i-lucide-download',
                    color: 'info',
                    duration: 0
                  })
                }
              })
              .catch((err: unknown) => {
                console.error('Failed to download/install update', err)
                toast.remove(currentToastId)
                toast.add({
                  title: 'Update failed',
                  description: 'Could not download or install the update. Please try again later.',
                  icon: 'i-lucide-circle-x',
                  color: 'error',
                  duration: 0
                })
              })
          }
        }
      ]
    })
  }

  const checkForUpdates = async (options: { silent?: boolean } = {}): Promise<void> => {
    checking.value = true

    let update
    try {
      update = await check()
    } catch (err) {
      console.error('Failed to check for updates', err)
      if (!options.silent) {
        toast.add({
          title: 'Update check failed',
          description: 'Could not reach the update server.',
          icon: 'i-lucide-circle-x',
          color: 'error'
        })
      }
      checking.value = false
      return
    }

    checking.value = false

    if (!update) {
      if (!options.silent) {
        toast.add({ title: "You're up to date", icon: 'i-lucide-check', color: 'success' })
      }
      return
    }

    showUpdateToast(update)
  }

  onMounted(() => {
    if (hasCheckedOnLaunch) return
    hasCheckedOnLaunch = true
    void checkForUpdates({ silent: true })
  })

  return { checking, checkForUpdates }
}
