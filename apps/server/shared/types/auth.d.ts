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
    desktopAuthRequestId: string
  }
}

export {}
