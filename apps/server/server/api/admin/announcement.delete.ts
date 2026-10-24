import { clearNotice } from '~~/server/utils/serverNotice'

export default defineEventHandler(async (event) => {
  await requireAdmin(event)

  clearNotice()

  return { success: true }
})
