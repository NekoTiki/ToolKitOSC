import { listLiveGenerations } from '~~/server/utils/ai/liveGenerations'

// Recovery for the stats dashboard's live "in progress" rows (see liveGenerations.ts) - called on
// mount and on every /admin/ws (re)connect, since a generation that was already running before
// either of those happened would otherwise never show up as live at all (only the eventual
// 'generation-logged' push once it finishes, same as before this feature existed).
export default defineEventHandler(async (event) => {
  await requireAdmin(event)

  return { attempts: listLiveGenerations() }
})
