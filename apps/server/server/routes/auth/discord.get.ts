export default defineOAuthDiscordEventHandler({
  config: {},
  async onSuccess(event, { user }) {
    const { id, user: previous, secure } = await getUserSession(event)

    await replaceUserSession(event, {
      user: {
        discord: {
          id: user.id,
          avatar: `https://cdn.discordapp.com/avatars/${user.id}/${user.avatar}.png`,
          name: user.global_name
        },
        username: user.username,
        email: user.email
      },
      secure: {
        ...secure,
        desktopAuthRequestId: undefined,
        // Only a guest session is worth linking; switching from one Discord account to another
        // shouldn't hand the old account's history to the new one.
        guestId: previous?.discord ? undefined : id
      }
    })

    if (secure?.desktopAuthRequestId) {
      return sendRedirect(event, `/auth/desktop?requestId=${secure.desktopAuthRequestId}`)
    }

    return sendRedirect(event, '/connect')
  },
  // Optional, will return a json error and 401 status code by default
  onError(event, error) {
    console.error('GitHub OAuth error:', error)
    return sendRedirect(event, '/')
  }
})
