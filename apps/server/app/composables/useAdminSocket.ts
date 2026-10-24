// The dashboard's live connection to /admin/ws (see server/routes/admin/ws.ts), opened on mount
// and closed on unmount. `onOpen` runs on every (re)connect, the moment to reload anything that
// may have been missed while disconnected.
export function useAdminSocket<T extends { type: string }>(onMessage: (data: T) => void, onOpen?: () => void): void {
  const { adminFetch } = useAdminApi()

  let ws: WebSocket | null = null
  let reconnectTimeout: ReturnType<typeof setTimeout> | undefined
  let closed = false

  async function connect(): Promise<void> {
    try {
      // The WS upgrade itself can't go through requireUserSession the way this fetch does, so this
      // ticket - minted over an authenticated REST call - is the actual auth check (see the
      // server's adminWsTickets.ts).
      const { ticket } = await adminFetch<{ ticket: string }>('/api/admin/ws-ticket')
      const protocol = location.protocol === 'https:' ? 'wss:' : 'ws:'

      if (closed) return

      ws = new WebSocket(`${protocol}//${location.host}/admin/ws?ticket=${ticket}`)

      ws.addEventListener('open', () => onOpen?.())

      ws.addEventListener('message', (event: MessageEvent<string>) => {
        try {
          onMessage(JSON.parse(event.data) as T)
        } catch (err) {
          console.error('[admin-ws] failed to apply message:', err)
        }
      })

      ws.addEventListener('close', () => {
        ws = null
        // Best-effort reconnect - a dashboard tab left open shouldn't need a manual reload just
        // because the connection dropped once (a server restart, a network blip).
        if (!closed) reconnectTimeout = setTimeout(connect, 5000)
      })
    } catch {
      if (!closed) reconnectTimeout = setTimeout(connect, 5000)
    }
  }

  onMounted(connect)
  onBeforeUnmount(() => {
    closed = true
    clearTimeout(reconnectTimeout)
    ws?.close()
    ws = null
  })
}
