import { check } from '@tauri-apps/plugin-updater'
import type { Ref } from 'vue'
import { onMounted, ref } from 'vue'

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
    const toastId = `app-update-${update.version}`

    toast.add({
      id: toastId,
      title: `Update available: v${update.version}`,
      description: 'Restarting will download and install the update.',
      icon: 'i-lucide-download',
      color: 'info',
      duration: 0,
      close: false,
      actions: [
        {
          label: 'Update & Restart',
          color: 'info',
          onClick: () => {
            toast.update(toastId, {
              description: 'Downloading update…',
              actions: []
            })

            // On Windows the installer exits and relaunches the app itself once installed, so
            // there's nothing left to do here if that succeeds.
            update
              .downloadAndInstall((event) => {
                if (event.event === 'Finished') {
                  toast.update(toastId, { description: 'Installing update…' })
                }
              })
              .catch((err: unknown) => {
                console.error('Failed to download/install update', err)
                toast.update(toastId, {
                  title: 'Update failed',
                  description: 'Could not download or install the update. Please try again later.',
                  color: 'error',
                  close: true
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

  onMounted(() => void checkForUpdates({ silent: true }))

  return { checking, checkForUpdates }
}
