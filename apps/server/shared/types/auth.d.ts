declare module '#auth-utils' {
  interface User {
    discord?: {
      id: string
      avatar: string
      name: string
    }
    userDefinedDisplayName?: string
    username: string
    email: string
  }

  // Required augmentation point for nuxt-auth-utils' session typing, even with nothing to add today.
  // eslint-disable-next-line @typescript-eslint/no-empty-object-type
  interface UserSession {}

  interface SecureSessionData {
    // Optional: only set while a desktop OAuth handoff is in flight, cleared once it completes.
    desktopAuthRequestId?: string
    // The guest session id this viewer had before logging in with Discord (login gives them a new
    // one). Sent to the host in the client list so it can fold their guest history into their
    // Discord account.
    guestId?: string
  }
}

export {}
