import { liveRooms } from '~~/server/utils/liveRooms'

export default defineEventHandler(async (event) => {
  await requireAdmin(event)

  return { rooms: await liveRooms() }
})
