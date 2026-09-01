import jwt from 'jsonwebtoken'

import { sendTokenToPeer } from '~~/server/routes/auth/desktop/ws'

export default defineEventHandler(async (event) => {
  const { user } = await requireUserSession(event)
  const { requestId } = getQuery(event)

  const runtimeConfig = useRuntimeConfig()

  if (!user.discord) return { success: false }

  const token = jwt.sign({ sub: user.discord.id, user }, runtimeConfig.session.password, {
    expiresIn: '4 Weeks'
  })

  return { success: sendTokenToPeer(requestId as string, token) }
})
