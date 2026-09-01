export default defineEventHandler(async (event) => {
  const { user } = await getUserSession(event)
  const { peerId } = getQuery(event)

  if (user) {
    return sendRedirect(event, `/auth/desktop?requestId=${peerId}`)
  }

  await setUserSession(event, {
    secure: {
      desktopAuthRequestId: peerId as string
    }
  })

  return sendRedirect(event, '/auth/discord')
})
