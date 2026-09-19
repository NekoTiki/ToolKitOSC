// The desktop client's webview runs on its own Vite origin (http://localhost:5173 in dev; a
// tauri://-style origin in a built release) while this server runs on its own (localhost:3000) -
// genuinely cross-origin, unlike the WS connection host.ts/ws/[ws].ts use (WS handshakes aren't
// subject to CORS the same way). A plain fetch() with a custom `Authorization` header triggers a
// preflight OPTIONS this server would otherwise never answer, so the browser blocks the real
// request before it's even sent - no server-side error, no entry in this route's own logs, just a
// silently empty response client-side. Scoped to /api/ai/* only, not applied globally: those
// routes carry no cookie/session state (see verifyDesktopToken - a bearer token, not a cookie), so
// a permissive origin here doesn't widen what any other route on this server exposes.
export default defineEventHandler((event) => {
  if (!event.path.startsWith('/api/ai/')) return

  setResponseHeaders(event, {
    'Access-Control-Allow-Origin': '*',
    'Access-Control-Allow-Methods': 'GET, POST, OPTIONS',
    'Access-Control-Allow-Headers': 'Content-Type, Authorization'
  })

  if (event.method === 'OPTIONS') return ''
})
