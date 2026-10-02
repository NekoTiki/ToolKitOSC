// AI access management shared by the admin Users list and each user's page: every change goes
// through the existing /api/admin/users endpoints, shows a toast, then asks the caller to reload.
export interface AccessEntry {
  discordId: string
  username: string | null
  displayName: string | null
  avatarUrl: string | null
  lastSeenAt: string
  hasAccess: boolean
  canSelectModel: boolean
  grantedBy: string | null
  grantedAt: string | null
  note: string | null
  credits: { remaining: number } | null
}

export const accessEntryName = (entry: Pick<AccessEntry, 'displayName' | 'username'>): string => entry.displayName || entry.username || 'Unknown'

export function useAdminUserAccess(reload: () => Promise<unknown>): {
  grant: (discordId: string, options?: { canSelectModel?: boolean; note?: string }) => Promise<void>
  revoke: (discordId: string) => Promise<void>
  setCanSelectModel: (discordId: string, canSelectModel: boolean) => Promise<void>
  saveNote: (entry: AccessEntry, note: string) => Promise<void>
  addCredits: (discordId: string, amount: number) => Promise<void>
} {
  const { adminFetch } = useAdminApi()
  const toast = useToast()

  // POST upserts, so it also works for a Discord ID that has never signed in yet.
  const grant = async (discordId: string, options: { canSelectModel?: boolean; note?: string } = {}): Promise<void> => {
    await adminFetch('/api/admin/users', {
      method: 'POST',
      body: { discordId: discordId.trim(), canSelectModel: options.canSelectModel ?? false, note: options.note?.trim() || undefined }
    })
    toast.add({ title: 'Access granted', icon: 'i-lucide-user-plus', color: 'success' })
    await reload()
  }

  const revoke = async (discordId: string): Promise<void> => {
    await adminFetch(`/api/admin/users/${discordId}`, { method: 'DELETE' })
    toast.add({ title: 'Access revoked', icon: 'i-lucide-user-minus' })
    await reload()
  }

  const setCanSelectModel = async (discordId: string, canSelectModel: boolean): Promise<void> => {
    await adminFetch(`/api/admin/users/${discordId}`, { method: 'PATCH', body: { canSelectModel } })
    toast.add({ title: canSelectModel ? 'Model choice allowed' : 'Model choice turned off', icon: 'i-lucide-cpu' })
    await reload()
  }

  // The note has no endpoint of its own; re-granting with the same settings upserts it.
  const saveNote = async (entry: AccessEntry, note: string): Promise<void> => {
    await adminFetch('/api/admin/users', { method: 'POST', body: { discordId: entry.discordId, canSelectModel: entry.canSelectModel, note: note.trim() || undefined } })
    toast.add({ title: 'Note saved', icon: 'i-lucide-check', color: 'success' })
    await reload()
  }

  // A bonus on top of today's pool only (UTC day), per the credits endpoint.
  const addCredits = async (discordId: string, amount: number): Promise<void> => {
    await adminFetch(`/api/admin/users/${discordId}/credits`, { method: 'POST', body: { amount } })
    toast.add({ title: `Added ${amount} credits for today`, icon: 'i-lucide-credit-card', color: 'success' })
    await reload()
  }

  return { grant, revoke, setCanSelectModel, saveNote, addCredits }
}
