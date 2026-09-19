// Comma-separated Discord ids (NUXT_AI_ADMIN_DISCORD_IDS) - not stored in the database, so
// promoting/demoting an admin is a redeploy-time config change, not something an admin could
// accidentally do to themselves (or each other) through the dashboard.
function adminIds(): string[] {
  return useRuntimeConfig()
    .aiAdminDiscordIds.split(',')
    .map((id) => id.trim())
    .filter(Boolean)
}

export function isAdmin(discordId: string | undefined): boolean {
  return !!discordId && adminIds().includes(discordId)
}
