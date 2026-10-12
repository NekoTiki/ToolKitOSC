import { router } from '@renderer/router'
import type { Update } from '@tauri-apps/plugin-updater'
import { check } from '@tauri-apps/plugin-updater'
import type { Ref, ShallowRef } from 'vue'
import { nextTick, onMounted, ref, shallowRef } from 'vue'

// Guards the silent launch check below so it only ever runs once app-wide. Every useAppUpdater()
// caller registers its own onMounted check - without this guard each would fire check(), and each
// would then toast.add() an "Update available" toast with the same id, landing two entries with
// the same key in the toasts array that Vue has to reconcile - which looks exactly like the toast
// flashing/closing instantly.
let hasCheckedOnLaunch = false

// True for `tauri dev` (Vite serving the app), false for every `vite build`, including releases.
// Fixed at build time, so a release can't lose its update checks. A dev build never installs an
// update: the installer would replace the released app, not this build.
const DEV_BUILD = import.meta.env.DEV

// The update the last check found, if any, so Settings › About can offer it next to its notes.
const pendingUpdate = shallowRef<Update>()
const installing = ref(false)

// Opens Settings › About and scrolls to the notes of the update (or the release notes when the
// section isn't there yet). The scroll matters when you're already on About - e.g. after pressing
// its Check for updates button - where the navigation alone does nothing.
async function showWhatsNew(): Promise<void> {
  await router.push({ name: 'settings', params: { section: 'about' } })
  await nextTick()
  const target = document.getElementById('update-notes') ?? document.getElementById('release-notes')
  target?.scrollIntoView({ behavior: 'smooth', block: 'start' })
}

// Downloads and installs `update`, then the app restarts. Used by the "Update available" toast and
// by Settings › About's Update & Restart button. Takes the caller's toast API: useToast() has to
// run during setup, not from a click handler.
function installUpdate(update: Update, toast: ReturnType<typeof useToast>): void {
  // Every state below gets its own fresh toast id instead of reusing/merging one - reusing an id
  // (via add()'s mergeDuplicate or update()) kept closing the toast out from under these
  // transitions, so each message now gets its own component instance rather than inheriting
  // whatever internal state the previous one left behind.
  toast.remove(`app-update-${update.version}`)
  installing.value = true

  // Tracks whichever of the toasts below is currently showing, so each step can remove exactly
  // that one before adding the next - downloadAndInstall can still reject after its 'Finished'
  // event already fired (e.g. a bad signature is only caught once the full download's in hand),
  // so the catch below can't assume it's cleaning up the "downloading" toast specifically.
  let currentToastId = `app-update-progress-${Date.now()}`

  toast.add({
    id: currentToastId,
    title: 'Downloading update…',
    icon: 'i-lucide-download',
    color: 'info',
    duration: 0
  })

  // On Windows the installer exits and relaunches the app itself once installed, so there's
  // nothing left to do here if that succeeds.
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
      installing.value = false
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

// Called both unconditionally from App.vue (silent - see useLogRetention.ts for the same
// call-once-on-mount pattern) and on demand from Settings → About's "Check for updates" button
// (not silent - that click deserves feedback even when there's nothing to install).
export function useAppUpdater(): {
  checking: Ref<boolean>
  installing: Ref<boolean>
  pendingUpdate: ShallowRef<Update | undefined>
  installPendingUpdate: () => void
  checkForUpdates: (options?: { silent?: boolean }) => Promise<void>
} {
  const toast = useToast()
  const checking = ref(false)

  const showUpdateToast = (update: Update): void => {
    const availableToastId = `app-update-${update.version}`

    // Opens Settings › About, which lists what changed since your version and has its own
    // Update & Restart button - toast actions always close the toast.
    const whatsNewAction = {
      label: "What's new",
      color: 'neutral' as const,
      variant: 'subtle' as const,
      onClick: () => void showWhatsNew()
    }

    if (DEV_BUILD) {
      toast.add({
        id: availableToastId,
        title: `Update available: v${update.version}`,
        description: "Development builds don't install updates. Install it from a release build.",
        icon: 'i-lucide-download',
        color: 'info',
        actions: [whatsNewAction]
      })
      return
    }

    toast.add({
      id: availableToastId,
      title: `Update available: v${update.version}`,
      description: 'Restarting will download and install the update.',
      icon: 'i-lucide-download',
      color: 'info',
      duration: 0,
      actions: [
        whatsNewAction,
        {
          label: 'Update & Restart',
          color: 'info',
          onClick: () => installUpdate(update, toast)
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
    pendingUpdate.value = update ?? undefined

    if (!update) {
      if (!options.silent) {
        toast.add({ title: "You're up to date", icon: 'i-lucide-check', color: 'success' })
      }
      return
    }

    showUpdateToast(update)
  }

  onMounted(() => {
    // No silent check at launch in development builds; Settings › About can still check by hand.
    if (DEV_BUILD || hasCheckedOnLaunch) return
    hasCheckedOnLaunch = true
    void checkForUpdates({ silent: true })
  })

  const installPendingUpdate = (): void => {
    if (pendingUpdate.value && !DEV_BUILD) installUpdate(pendingUpdate.value, toast)
  }

  return { checking, installing, pendingUpdate, installPendingUpdate, checkForUpdates }
}
