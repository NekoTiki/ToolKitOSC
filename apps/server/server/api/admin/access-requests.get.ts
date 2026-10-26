import { listPendingRequests } from '~~/server/utils/ai/access'

export default defineEventHandler(async (event) => {
  await requireAdmin(event)

  return { requests: await listPendingRequests() }
})
