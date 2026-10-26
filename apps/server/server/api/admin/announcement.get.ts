import { currentNotice } from '~~/server/utils/serverNotice'

export default defineEventHandler(async (event) => {
  await requireAdmin(event)

  return { notice: currentNotice() }
})
