// "Only people you let in" (the host's ShareSettingsMessage.approval): who gets into the share
// page, who has to sign in first, and who waits for the host. Pure, so it's tested on its own.
export type AccessDecision = 'admit' | 'sign-in' | 'wait'

export function accessDecision(settings: { approval?: boolean; approved?: string[] } | undefined, discordId: string | undefined): AccessDecision {
  if (!settings?.approval) return 'admit'
  // Approval is by Discord account, so a guest can't be let in.
  if (!discordId) return 'sign-in'

  return settings.approved?.includes(discordId) ? 'admit' : 'wait'
}
