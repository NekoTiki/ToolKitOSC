// Posts to the Discord webhook in NUXT_ALERT_DISCORD_WEBHOOK when something needs an admin. Each
// kind of alert (`key`) waits 30 minutes before it can post again, so a lasting problem is one
// message, not one a second. Does nothing without a webhook.
const COOLDOWN_MS = 30 * 60 * 1000

const lastSent = new Map<string, number>()

export function sendAlert(key: string, text: string): void {
  const webhook = useRuntimeConfig().alertDiscordWebhook

  if (!webhook || Date.now() - (lastSent.get(key) ?? 0) < COOLDOWN_MS) return

  lastSent.set(key, Date.now())
  void fetch(webhook, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ content: `⚠️ ToolKitOSC server: ${text}`, allowed_mentions: { parse: [] } })
  }).catch((error) => console.error('[alerts] webhook failed:', error))
}
